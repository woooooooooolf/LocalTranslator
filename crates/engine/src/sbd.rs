//! MiniSBD inference and boundary decoding. Preprocessing and long-window behavior
//! follow the installed MiniSBD/Stanza runtime; models and weights are unchanged.
use ort::{session::Session, value::Tensor};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::OnceLock,
};

pub struct SentenceDetector {
    session: Session,
    vocab: HashMap<String, i64>,
    features: Vec<String>,
    mask: fancy_regex::Regex,
    dictionary: Option<Dictionary>,
    skip_newline: bool,
}
struct Dictionary {
    words: HashSet<String>,
    prefixes: HashSet<String>,
    suffixes: HashSet<String>,
    width: usize,
}
#[derive(Clone)]
struct Unit {
    ch: char,
    begin: usize,
    end: usize,
}
fn numeric(ch: char) -> bool {
    static DIGIT: OnceLock<regex::Regex> = OnceLock::new();
    DIGIT
        .get_or_init(|| regex::Regex::new(r"^\p{Nd}$").unwrap())
        .is_match(&ch.to_string())
}
impl SentenceDetector {
    pub fn load(path: &Path) -> Result<Self, String> {
        static ENV: OnceLock<Result<(), String>> = OnceLock::new();
        ENV.get_or_init(|| {
            ort::init()
                .with_name("Jingyi")
                .with_telemetry(false)
                .commit()
                .map(|_| ())
                .map_err(|_| "分句运行时初始化失败".into())
        })
        .clone()?;
        let bytes = std::fs::read(path).map_err(|_| "缺少本地 MiniSBD 模型")?;
        let session = Session::builder()
            .and_then(|s| s.with_intra_threads(2))
            .and_then(|s| s.with_inter_threads(1))
            .and_then(|s| s.with_log_level(ort::logging::LogLevel::Fatal))
            .and_then(|s| s.commit_from_memory(&bytes))
            .map_err(|_| "无法加载 MiniSBD 模型")?;
        let (config, vocab, lexicon) = {
            let metadata = session.metadata().map_err(|_| "分句模型缺少配置")?;
            let config: Value = serde_json::from_str(
                &metadata
                    .custom("config")
                    .map_err(|_| "分句配置读取失败")?
                    .ok_or("分句配置缺失")?,
            )
            .map_err(|_| "分句配置无效")?;
            let vocab: Value = serde_json::from_str(
                &metadata
                    .custom("vocab")
                    .map_err(|_| "分句词表读取失败")?
                    .ok_or("分句词表缺失")?,
            )
            .map_err(|_| "分句词表无效")?;
            let lexicon: Value = serde_json::from_str(
                &metadata
                    .custom("lexicon")
                    .map_err(|_| "分句词典读取失败")?
                    .unwrap_or_else(|| "null".into()),
            )
            .map_err(|_| "分句词典无效")?;
            (config, vocab, lexicon)
        };
        let skip_newline = config["skip_newline"].as_bool().unwrap_or(false);
        let dictionary = if config["use_dictionary"].as_bool().unwrap_or(false) {
            let width = config["num_dict_feat"].as_u64().ok_or("分句词典宽度无效")? as usize;
            if width > 16 {
                return Err("分句词典宽度不受支持".into());
            }
            let mut dict = Dictionary {
                words: HashSet::new(),
                prefixes: HashSet::new(),
                suffixes: HashSet::new(),
                width,
            };
            for word in lexicon.as_array().ok_or("分句词典缺失")? {
                let word = word.as_str().ok_or("分句词典格式无效")?;
                let points: Vec<_> = word.char_indices().map(|(i, _)| i).collect();
                if points.len() < 2 {
                    continue;
                }
                dict.words.insert(word.to_string());
                for point in points.into_iter().skip(1) {
                    dict.prefixes.insert(word[..point].to_owned());
                    dict.suffixes.insert(word[point..].to_owned());
                }
            }
            Some(dict)
        } else {
            None
        };
        let features: Vec<String> =
            serde_json::from_value(config["feat_funcs"].clone()).map_err(|_| "分句特征无效")?;
        let mut ordered: Vec<String> = features
            .iter()
            .filter(|s| !matches!(s.as_str(), "end_of_para" | "start_of_para"))
            .cloned()
            .collect();
        if features.iter().any(|s| s == "end_of_para") {
            ordered.push("end_of_para".into());
        }
        if features.iter().any(|s| s == "start_of_para") {
            ordered.push("start_of_para".into());
        }
        if ordered.is_empty()
            || ordered.iter().any(|s| {
                ![
                    "space_before",
                    "capitalized",
                    "numeric",
                    "end_of_para",
                    "start_of_para",
                ]
                .contains(&s.as_str())
            })
        {
            return Err("此分句模型特征尚不受支持".into());
        }
        let vocab =
            serde_json::from_value(vocab["_unit2id"].clone()).map_err(|_| "分句词表无效")?;
        // Preserve URLs/email tokens before decoding model boundary labels.
        let mask = fancy_regex::Regex::new(r#"(?:[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9.-]*[a-zA-Z0-9])?\.[a-zA-Z]{2,})|(?:https?://[^\s\"]+|www\.[^\s\"]+|[a-zA-Z0-9]+\.(?:gov|org|edu|net|com|co)(?:\.[^\s\"]{2,})?)"#).map_err(|_| "分句规则初始化失败")?;
        Ok(Self {
            session,
            vocab,
            features: ordered,
            mask,
            dictionary,
            skip_newline,
        })
    }
    fn predict(
        &mut self,
        units: &[Unit],
        offset: usize,
        count: usize,
        real_len: usize,
    ) -> Result<Vec<usize>, String> {
        let mut ids = vec![0i64; count];
        let width = self.features.len() + self.dictionary.as_ref().map_or(0, |d| d.width * 2);
        let mut feats = vec![0f32; count * width];
        for i in 0..count {
            let index = offset + i;
            if index >= real_len {
                break;
            }
            let ch = units[index].ch;
            ids[i] = *self.vocab.get(&ch.to_string()).unwrap_or(&1);
            for (j, name) in self.features.iter().enumerate() {
                let value = match name.as_str() {
                    "space_before" => ch == ' ',
                    "capitalized" => ch.is_uppercase(),
                    "numeric" => numeric(ch),
                    "end_of_para" => index + 1 == real_len,
                    "start_of_para" => index == 0,
                    _ => false,
                };
                feats[i * width + j] = u8::from(value) as f32;
            }
            if let Some(dict) = &self.dictionary {
                let (mut forward, mut backward) = (ch.to_string(), ch.to_string());
                let (mut prefix, mut suffix) = (true, true);
                for window in 1..=dict.width {
                    if index + window < real_len && prefix {
                        forward.extend(units[index + window].ch.to_lowercase());
                        feats[i * width + self.features.len() + window - 1] =
                            u8::from(dict.words.contains(&forward)) as f32;
                        if !dict.prefixes.contains(&forward) {
                            prefix = false;
                        }
                    }
                    if index >= window && suffix {
                        backward =
                            format!("{}{}", units[index - window].ch.to_lowercase(), backward);
                        feats[i * width + self.features.len() + dict.width + window - 1] =
                            u8::from(dict.words.contains(&backward)) as f32;
                        if !dict.suffixes.contains(&backward) {
                            suffix = false;
                        }
                    }
                    if !prefix && !suffix {
                        break;
                    }
                }
            }
        }
        let ids = Tensor::from_array(([1, count], ids)).map_err(|_| "分句输入无效")?;
        let feats = Tensor::from_array(([1, count, width], feats)).map_err(|_| "分句特征无效")?;
        let outputs = self
            .session
            .run(ort::inputs! {"units" => ids, "features" => feats})
            .map_err(|_| "本地分句失败")?;
        let (shape, data) = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(|_| "分句结果无效")?;
        if shape.len() != 3
            || shape[0] != 1
            || shape[1] as usize != count
            || ![3, 5].contains(&shape[2])
        {
            return Err("分句结果尺寸无效".into());
        }
        Ok(data
            .chunks_exact(shape[2] as usize)
            .map(|row| {
                row.iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1).then(b.0.cmp(&a.0)))
                    .unwrap()
                    .0
            })
            .collect())
    }
    pub fn sentences(&mut self, text: &str) -> Result<Vec<String>, String> {
        let text = text.trim_end();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        let mut units: Vec<Unit> = Vec::new();
        for (begin, original) in text.char_indices() {
            if self.skip_newline && original == '\n' {
                continue;
            }
            let ch = if original.is_whitespace() {
                ' '
            } else {
                original
            };
            if ch == ' ' && units.last().is_some_and(|u| u.ch == ' ') {
                continue;
            }
            units.push(Unit {
                ch,
                begin,
                end: begin + original.len_utf8(),
            });
        }
        let n = units.len();
        if n == 0 {
            return Ok(Vec::new());
        }
        let mut predictions = if n < 1000 {
            self.predict(&units, 0, n + 1, n)?[..n].to_vec()
        } else {
            let (mut offset, mut all) = (0, Vec::with_capacity(n));
            while offset < n {
                let count = (n - offset).min(1000);
                let pred = self.predict(&units, offset, count, n)?;
                let advance = if n - offset <= 1000 {
                    count
                } else {
                    pred.iter()
                        .rposition(|p| matches!(p, 2 | 4))
                        .map_or(count, |p| p + 1)
                };
                all.extend_from_slice(&pred[..advance]);
                offset += advance;
            }
            all
        };
        predictions[n - 1] = if predictions[n - 1] > 2 { 4 } else { 2 };
        let normalized: String = units.iter().map(|u| u.ch).collect();
        let offsets: Vec<usize> = normalized
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(normalized.len()))
            .collect();
        for matched in self.mask.find_iter(&normalized) {
            let matched = matched.map_err(|_| "分句规则执行失败")?;
            let from = offsets.partition_point(|x| *x < matched.start());
            let to = offsets.partition_point(|x| *x < matched.end());
            if to <= from {
                continue;
            }
            predictions[from..to - 1].fill(0);
            if predictions[to - 1] == 0 {
                predictions[to - 1] = 1;
            }
        }
        let (mut result, mut sentence_start, mut token_start) = (Vec::new(), None, 0);
        for (i, pred) in predictions.iter().enumerate() {
            if *pred == 0 {
                continue;
            }
            let token = &units[token_start..=i];
            if let Some(first) = token.iter().find(|u| u.ch != ' ') {
                sentence_start.get_or_insert(first.begin);
                if matches!(pred, 2 | 4) {
                    let end = token.iter().rfind(|u| u.ch != ' ').unwrap().end;
                    result.push(text[sentence_start.take().unwrap()..end].to_string());
                }
            }
            token_start = i + 1;
        }
        if let Some(start) = sentence_start {
            result.push(text[start..].trim_end().to_string());
        }
        Ok(result)
    }
}
