"""Development comparison only: original Python CTranslate2 versus static C++ CPU."""
import json
from pathlib import Path
import subprocess
import argparse
import ctranslate2
import sentencepiece

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = {
    'en_zh': ['Hello, world!', 'This translation runs locally on the CPU.', 'The file is stored on your device.',
              'Please check the USB connection.', 'The temperature is 23.5 degrees.', 'Privacy matters.',
              'Do not save this text automatically.', 'A short sentence. Another sentence.'],
    'zh_en': ['你好，世界！', '这个翻译工具在本地 CPU 上运行。', '文件保存在你的设备上。',
              '请检查 USB 连接。', '温度为23.5度。', '隐私很重要。',
              '不要自动保存这段文字。', '这是第一句话。这是第二句话。'],
}
matched = 0
total = 0
differences = []
parser = argparse.ArgumentParser()
parser.add_argument('--strict', action='store_true', help='Require exact token parity, in addition to successful inference')
args = parser.parse_args()
for direction, samples in FIXTURES.items():
    package = ROOT / 'runtime/packages' / ('translate-' + direction + '-1_9')
    tokenizer = sentencepiece.SentencePieceProcessor(model_file=str(package/'sentencepiece.model'))
    baseline = ctranslate2.Translator(str(package/'model'), device='cpu')
    native = subprocess.Popen([str(ROOT/'.cache/native/build/jingyi-native-check.exe'), str(package/'model')],
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              text=True, encoding='utf-8')
    try:
        for text in samples:
            tokens = [tokenizer.encode(text, out_type=str)]
            expected = baseline.translate_batch(tokens, replace_unknowns=True, max_batch_size=32,
                        batch_type='tokens', beam_size=4, num_hypotheses=1, length_penalty=0.2)[0].hypotheses[0]
            native.stdin.write(json.dumps(tokens) + '\n'); native.stdin.flush()
            line = native.stdout.readline()
            if not line:
                raise AssertionError(f'Native engine stopped: {native.poll()}, stderr={native.stderr.read()}')
            actual = json.loads(line)
            assert actual and all(isinstance(piece, str) for piece in actual)
            total += 1
            if actual == expected:
                matched += 1
            else:
                differences.append((direction, text, tokenizer.decode_pieces(expected), tokenizer.decode_pieces(actual)))
    finally:
        native.stdin.close()
        try: native.wait(timeout=20)
        except subprocess.TimeoutExpired: native.kill(); native.wait()
    assert native.returncode == 0
print(f'Native CPU inference succeeded: {total}/{total}; exact token outputs: {matched}/{total}; Unicode model paths supported.')
for difference in differences:
    print('DIFFERENCE:',json.dumps(difference,ensure_ascii=False))
if args.strict and differences:
    raise SystemExit(1)
