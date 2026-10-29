//! Native document translation. Read source bytes without staging a source copy;
//! only an explicit user-selected output is written, atomically after success.
use kuchikiki::traits::*;
use quick_xml::{events::Event, Reader};
use std::{
    fs,
    io::{Cursor, Read, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

const MAX_FILE: usize = 50 * 1024 * 1024;
const MAX_XML: u64 = 20 * 1024 * 1024;
const MAX_EXPANDED: u64 = 150 * 1024 * 1024;
type Translator<'a> = &'a mut dyn FnMut(&str) -> Result<String, String>;

fn decode(bytes: &[u8]) -> Result<String, String> {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(text.trim_start_matches('\u{feff}').to_owned());
    }
    let (text, _, errors) = encoding_rs::GB18030.decode(bytes);
    if errors {
        Err("无法解码文档，请转换为 UTF-8 后重试。".into())
    } else {
        Ok(text.into_owned())
    }
}
fn translate_trimmed(text: &str, translate: Translator<'_>) -> Result<String, String> {
    let core = text.trim();
    if core.is_empty() {
        return Ok(text.to_owned());
    }
    let leading = text.len() - text.trim_start().len();
    let trailing = text.trim_end().len();
    Ok(format!(
        "{}{}{}",
        &text[..leading],
        translate(core)?,
        &text[trailing..]
    ))
}

#[derive(Clone, Copy)]
enum XmlKind {
    Office,
    OpenDocument,
    Xhtml,
}
struct TextNode {
    start: usize,
    end: usize,
    text: String,
}
struct Group {
    depth: usize,
    nodes: Vec<TextNode>,
}
fn flush(
    group: &mut Group,
    edits: &mut Vec<(usize, usize, String)>,
    translate: Translator<'_>,
    count: &mut usize,
) -> Result<(), String> {
    let text: String = group.nodes.iter().map(|n| n.text.as_str()).collect();
    if !text.trim().is_empty() {
        let translated = translate_trimmed(&text, translate)?;
        for (i, node) in group.nodes.iter().enumerate() {
            edits.push((
                node.start,
                node.end,
                if i == 0 {
                    quick_xml::escape::escape(&translated).into_owned()
                } else {
                    String::new()
                },
            ));
        }
        *count += 1;
    }
    group.nodes.clear();
    Ok(())
}
fn xml(bytes: &[u8], kind: XmlKind, translate: Translator<'_>) -> Result<(Vec<u8>, usize), String> {
    let mut content = std::str::from_utf8(bytes)
        .map_err(|_| "Office/EPUB XML 需要 UTF-8 编码")?
        .to_owned();
    let mut reader = Reader::from_str(&content);
    let (mut stack, mut groups, mut edits) =
        (Vec::<String>::new(), Vec::<Group>::new(), Vec::new());
    let mut count = 0;
    loop {
        let start = reader.buffer_position() as usize;
        let event = reader.read_event().map_err(|_| "文档 XML 格式无效")?;
        let end = reader.buffer_position() as usize;
        match event {
            Event::Start(event) => {
                let tag = String::from_utf8_lossy(event.local_name().as_ref()).into_owned();
                stack.push(tag.clone());
                if tag == "p"
                    || (matches!(kind, XmlKind::OpenDocument) && tag == "h")
                    || (matches!(kind, XmlKind::Xhtml)
                        && [
                            "div", "li", "td", "th", "h1", "h2", "h3", "h4", "h5", "h6", "body",
                        ]
                        .contains(&tag.as_str()))
                {
                    if let Some(group) = groups.last_mut() {
                        flush(group, &mut edits, translate, &mut count)?;
                    }
                    groups.push(Group {
                        depth: stack.len(),
                        nodes: Vec::new(),
                    });
                }
            }
            Event::End(_) => {
                if groups.last().is_some_and(|g| g.depth == stack.len()) {
                    flush(
                        &mut groups.pop().unwrap(),
                        &mut edits,
                        translate,
                        &mut count,
                    )?;
                }
                stack.pop();
            }
            Event::Empty(event) => {
                let tag = String::from_utf8_lossy(event.local_name().as_ref()).into_owned();
                if ["br", "tab", "line-break", "s"].contains(&tag.as_str()) {
                    if let Some(group) = groups.last_mut() {
                        flush(group, &mut edits, translate, &mut count)?;
                    }
                }
            }
            Event::Text(_) | Event::GeneralRef(_) | Event::CData(_) => {
                let visible = matches!(kind, XmlKind::OpenDocument)
                    || (matches!(kind, XmlKind::Xhtml)
                        && !stack.iter().any(|s| {
                            ["script", "style", "pre", "code", "head"].contains(&s.as_str())
                        }))
                    || (matches!(kind, XmlKind::Office) && stack.last().is_some_and(|t| t == "t"));
                if visible {
                    if let Some(group) = groups.last_mut() {
                        let raw = &content[start..end];
                        let text = if let Some(text) = raw
                            .strip_prefix("<![CDATA[")
                            .and_then(|r| r.strip_suffix("]]>"))
                        {
                            text.to_owned()
                        } else {
                            quick_xml::escape::unescape(raw)
                                .map_err(|_| "文档含不支持的 XML 实体")?
                                .into_owned()
                        };
                        group.nodes.push(TextNode { start, end, text });
                    }
                }
            }
            Event::Eof => break,
            _ => (),
        }
    }
    if !stack.is_empty() || !groups.is_empty() {
        return Err("文档 XML 未完整闭合".into());
    }
    edits.sort_by_key(|e| e.0);
    for (start, end, value) in edits.into_iter().rev() {
        content.replace_range(start..end, &value);
    }
    Ok((content.into_bytes(), count))
}

fn html(content: &str, translate: Translator<'_>) -> Result<(String, usize), String> {
    let document = kuchikiki::parse_html().one(content);
    let mut groups: Vec<(kuchikiki::NodeRef, Vec<kuchikiki::NodeRef>)> = Vec::new();
    for text in document.descendants().text_nodes() {
        let mut block = None;
        let mut skip = false;
        for ancestor in text.as_node().ancestors() {
            if let Some(element) = ancestor.as_element() {
                let tag = element.name.local.as_ref();
                if [
                    "script", "style", "code", "pre", "noscript", "title", "head",
                ]
                .contains(&tag)
                {
                    skip = true;
                    break;
                }
                if block.is_none()
                    && [
                        "p", "div", "li", "td", "th", "h1", "h2", "h3", "h4", "h5", "h6", "body",
                    ]
                    .contains(&tag)
                {
                    block = Some(ancestor.clone());
                }
            }
        }
        if skip {
            continue;
        }
        let block = block.unwrap_or_else(|| document.clone());
        if let Some((_, nodes)) = groups.iter_mut().find(|(parent, _)| *parent == block) {
            nodes.push(text.as_node().clone());
        } else {
            groups.push((block, vec![text.as_node().clone()]));
        }
    }
    let mut count = 0;
    for (_, nodes) in groups {
        let text: String = nodes
            .iter()
            .map(|n| n.as_text().unwrap().borrow().clone())
            .collect();
        if text.trim().is_empty() {
            continue;
        }
        count += 1;
        let translated = translate_trimmed(&text, translate)?;
        for (i, node) in nodes.into_iter().enumerate() {
            *node.as_text().unwrap().borrow_mut() = if i == 0 {
                translated.clone()
            } else {
                String::new()
            };
        }
    }
    let mut result = Vec::new();
    document
        .serialize(&mut result)
        .map_err(|_| "HTML 输出失败")?;
    Ok((
        String::from_utf8(result).map_err(|_| "HTML 编码失败")?,
        count,
    ))
}

fn subtitles(text: &str, translate: Translator<'_>) -> Result<(String, usize), String> {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let normalized = text.replace("\r\n", "\n");
    let separator = regex::Regex::new(r"\n{2,}").unwrap();
    let (mut result, mut start, mut count) = (String::new(), 0, 0);
    for end in separator
        .find_iter(&normalized)
        .map(|m| (m.start(), m.end()))
        .chain(std::iter::once((normalized.len(), normalized.len())))
    {
        let block = &normalized[start..end.0];
        let mut lines = block.splitn(3, '\n');
        let index = lines.next().unwrap_or("");
        let timing = lines.next().unwrap_or("");
        let caption = lines.next().unwrap_or("");
        if !block.trim().is_empty() {
            if index.trim().parse::<u64>().is_err()
                || !timing.contains("-->")
                || caption.trim().is_empty()
            {
                return Err("SRT 字幕结构无效".into());
            }
            result.push_str(index);
            result.push('\n');
            result.push_str(timing);
            result.push('\n');
            // Keep common subtitle style tags as markup instead of feeding them to NMT.
            if caption.contains('<') {
                let wrapper = format!("<root><p>{caption}</p></root>");
                let (translated, _) = xml(wrapper.as_bytes(), XmlKind::OpenDocument, translate)?;
                let value = String::from_utf8(translated).map_err(|_| "字幕编码失败")?;
                result.push_str(
                    value
                        .strip_prefix("<root><p>")
                        .and_then(|s| s.strip_suffix("</p></root>"))
                        .ok_or("字幕标签无效")?,
                );
            } else {
                result.push_str(&translate_trimmed(caption, translate)?);
            }
            count += 1;
        }
        result.push_str(&normalized[end.0..end.1]);
        start = end.1;
    }
    Ok((result.replace('\n', newline), count))
}

fn container(
    bytes: Vec<u8>,
    extension: &str,
    translate: Translator<'_>,
) -> Result<(Vec<u8>, usize), String> {
    let mut input = ZipArchive::new(Cursor::new(bytes)).map_err(|_| "文档压缩容器无效或已加密")?;
    if input.len() > 10000 {
        return Err("文档条目过多".into());
    }
    let mut output = ZipWriter::new(Cursor::new(Vec::new()));
    let (mut count, mut expanded) = (0, 0u64);
    for i in 0..input.len() {
        let mut file = input.by_index(i).map_err(|_| "无法读取文档条目")?;
        if file.enclosed_name().is_none() {
            return Err("文档包含不安全的路径".into());
        }
        expanded = expanded.checked_add(file.size()).ok_or("文档过大")?;
        if expanded > MAX_EXPANDED {
            return Err("文档解压体积超过限制".into());
        }
        let name = file.name().to_owned();
        let selected = match extension {
            "docx" => {
                name == "word/document.xml"
                    || (name.starts_with("word/")
                        && ["header", "footer", "footnotes", "endnotes"]
                            .iter()
                            .any(|prefix| name[5..].starts_with(prefix))
                        && name.ends_with(".xml"))
            }
            "pptx" => {
                (name.starts_with("ppt/slides/slide")
                    || name.starts_with("ppt/notesSlides/notesSlide"))
                    && name.ends_with(".xml")
            }
            "odt" | "odp" => name == "content.xml",
            "epub" => [".xhtml", ".html", ".htm"]
                .iter()
                .any(|suffix| name.to_lowercase().ends_with(suffix)),
            _ => false,
        };
        if !selected || file.is_dir() {
            output.raw_copy_file(file).map_err(|_| "复制文档资源失败")?;
            continue;
        }
        if file.size() > MAX_XML {
            return Err("文档 XML 条目超过限制".into());
        }
        let mut content = Vec::new();
        (&mut file)
            .take(MAX_XML + 1)
            .read_to_end(&mut content)
            .map_err(|_| "读取文档文本失败")?;
        if content.len() as u64 > MAX_XML {
            return Err("文档 XML 条目超过限制".into());
        }
        let (translated, changed) = if extension == "epub" {
            // XHTML remains XML to preserve EPUB's namespace/declaration requirements.
            xml(&content, XmlKind::Xhtml, translate)?
        } else {
            xml(
                &content,
                if ["odt", "odp"].contains(&extension) {
                    XmlKind::OpenDocument
                } else {
                    XmlKind::Office
                },
                translate,
            )?
        };
        let options = SimpleFileOptions::default()
            .compression_method(file.compression())
            .last_modified_time(file.last_modified().unwrap_or_default())
            .unix_permissions(file.unix_mode().unwrap_or(0o644));
        output
            .start_file(name, options)
            .map_err(|_| "创建译文条目失败")?;
        output
            .write_all(&translated)
            .map_err(|_| "写入译文条目失败")?;
        count += changed;
    }
    Ok((
        output
            .finish()
            .map_err(|_| "文档容器输出失败")?
            .into_inner(),
        count,
    ))
}

pub fn translate(
    input: &Path,
    output: &Path,
    mut translate: impl FnMut(&str) -> Result<String, String>,
) -> Result<(), String> {
    let input = input.canonicalize().map_err(|_| "找不到原文档")?;
    let extension = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !crate::FORMATS.contains(&extension.as_str()) {
        return Err("不支持此格式；PDF 翻译已移除。".into());
    }
    if !input.is_file() || input.metadata().map_err(|_| "无法读取原文档")?.len() > MAX_FILE as u64
    {
        return Err("单份文档最大支持 50 MB。".into());
    }
    if output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        != extension
    {
        return Err("请保留原文档扩展名".into());
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let parent = parent.canonicalize().map_err(|_| "译文目录不存在")?;
    let destination = parent.join(output.file_name().ok_or("译文文件名无效")?);
    if destination
        .canonicalize()
        .unwrap_or_else(|_| destination.clone())
        == input
    {
        return Err("不能覆盖原文件".into());
    }
    let bytes = fs::read(&input).map_err(|_| "读取原文档失败")?;
    let (result, count) = match extension.as_str() {
        "txt" => {
            let text = decode(&bytes)?;
            let translated = translate_trimmed(&text, &mut translate)?;
            (
                translated.into_bytes(),
                usize::from(!text.trim().is_empty()),
            )
        }
        "srt" => {
            let (s, c) = subtitles(&decode(&bytes)?, &mut translate)?;
            (s.into_bytes(), c)
        }
        "html" => {
            let (s, c) = html(&decode(&bytes)?, &mut translate)?;
            (s.into_bytes(), c)
        }
        _ => container(bytes, &extension, &mut translate)?,
    };
    if count == 0 {
        return Err("文档没有可翻译的文字".into());
    }
    let mut pending =
        tempfile::NamedTempFile::new_in(&parent).map_err(|_| "无法创建译文临时文件")?;
    pending
        .write_all(&result)
        .and_then(|_| pending.as_file().sync_all())
        .map_err(|_| "保存译文失败")?;
    pending
        .persist(&destination)
        .map_err(|_| "无法写入译文；请确认文件未被占用")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn word_runs_join_and_escaped_output_preserves_markup() {
        let source = br#"<w:document xmlns:w="urn:w"><w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Hello &amp; </w:t></w:r><w:r><w:t>world</w:t></w:r></w:p></w:document>"#;
        let (result, count) = xml(source, XmlKind::Office, &mut |s| {
            assert_eq!(s, "Hello & world");
            Ok("A < B & C".into())
        })
        .unwrap();
        let result = String::from_utf8(result).unwrap();
        assert_eq!(count, 1);
        assert!(result.contains("A &lt; B &amp; C"));
        assert!(result.contains("<w:b/>"));
    }
    #[test]
    fn subtitles_keep_timing_and_styles() {
        let (result, count) = subtitles(
            "1\r\n00:00:00,000 --> 00:00:02,000\r\n<i>Hello</i>\r\n\r\n",
            &mut |_| Ok("译文".into()),
        )
        .unwrap();
        assert_eq!(count, 1);
        assert!(result.contains("00:00:00,000 --> 00:00:02,000\r\n<i>译文</i>"));
    }
    #[test]
    fn html_leaves_scripts_and_code_untranslated() {
        let (result, _) = html(
            "<p>Hello <b>world</b></p><script>secret()</script><pre>code</pre>",
            &mut |s| {
                assert_eq!(s, "Hello world");
                Ok("译文".into())
            },
        )
        .unwrap();
        assert!(result.contains("secret()"));
        assert!(result.contains("<pre>code</pre>"));
        assert!(result.contains("译文"));
    }
    #[test]
    fn zip_untranslated_payload_and_source_remain_intact() {
        let folder = tempfile::tempdir().unwrap();
        let input = folder.path().join("test.docx");
        let output = folder.path().join("out.docx");
        let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
        archive
            .start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"<p><t>Hello</t></p>").unwrap();
        archive
            .start_file("word/media/keep.bin", SimpleFileOptions::default())
            .unwrap();
        archive.write_all(b"payload").unwrap();
        let source = archive.finish().unwrap().into_inner();
        fs::write(&input, &source).unwrap();
        translate(&input, &output, |_| Ok("译文".into())).unwrap();
        assert_eq!(fs::read(&input).unwrap(), source);
        let mut archive = ZipArchive::new(Cursor::new(fs::read(&output).unwrap())).unwrap();
        let mut payload = Vec::new();
        archive
            .by_name("word/media/keep.bin")
            .unwrap()
            .read_to_end(&mut payload)
            .unwrap();
        assert_eq!(payload, b"payload");
        assert!(translate(&input, &input, |_| Ok("译文".into())).is_err());
    }
}
