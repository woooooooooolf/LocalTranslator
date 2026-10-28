"""Real CPU integration checks against the relocated portable runtime."""
import hashlib
import json
import re
from pathlib import Path
import subprocess
import tempfile
import zipfile
import pymupdf

ROOT = Path(__file__).resolve().parent.parent
def main():
    log = ROOT / 'data/smoke.log'
    log.parent.mkdir(exist_ok=True)
    with log.open('w', encoding='utf-8') as errors:
        worker = subprocess.Popen([str(ROOT / 'runtime/py/python.exe'), '-u', '-B', '-X', 'utf8', str(ROOT / 'backend/worker.py')],
                                  stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=errors, text=True, encoding='utf-8', cwd=ROOT)
        def request(data, success=True):
            worker.stdin.write(json.dumps(data, ensure_ascii=False) + '\n')
            worker.stdin.flush()
            result = json.loads(worker.stdout.readline())
            assert result['ok'] == success, result
            print(json.dumps(result, ensure_ascii=False), flush=True)
            return result.get('data')
        try:
            info = request({'op': 'info'})
            assert {'en', 'zh'} <= {l['code'] for l in info['languages']}
            for source, target, text in [('en', 'zh', 'Hello, world!\nThis is an offline translation tool.'), ('zh', 'en', '你好，世界！\n这是一个离线翻译工具。')]:
                result = request({'op': 'text', 'source': source, 'target': target, 'text': text})
                assert result['text'].strip() and result['text'] != text
            request({'op': 'text', 'source': 'zh', 'target': 'zh', 'text': '你好'}, False)
            request({'op': 'text', 'source': 'en', 'target': 'zh', 'text': ''}, False)
            request({'op': 'text', 'source': 'en', 'target': 'zh', 'text': 'a' * 20001}, False)
            with tempfile.TemporaryDirectory(dir=ROOT / 'data', prefix='smoke-中文-') as folder:
                folder = Path(folder)
                txt = folder / '输入.txt'; txt.write_text('Hello, world!', encoding='utf-8')
                srt = folder / '字幕.srt'; srt.write_text('1\n00:00:00,000 --> 00:00:02,000\nHello, world!\n', encoding='utf-8')
                docx = folder / '文档.docx'
                pdf = folder / '文字层.pdf'
                with pymupdf.open() as document:
                    page = document.new_page()
                    page.insert_text((72, 72), 'Hello, world!')
                    document.save(pdf)
                with zipfile.ZipFile(docx, 'w') as archive:
                    archive.writestr('word/document.xml', '<?xml version="1.0"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>Hello, world!</w:t></w:r></w:p></w:body></w:document>')
                    archive.writestr('preserve.bin', b'unchanged')
                for path in [txt, srt, docx, pdf]:
                    before = hashlib.sha256(path.read_bytes()).digest()
                    destination = path.with_stem(path.stem + '-译文')
                    request({'op': 'file', 'source': 'en', 'target': 'zh', 'path': str(path), 'output': str(destination)})
                    assert destination.is_file() and destination.read_bytes() != path.read_bytes()
                    assert hashlib.sha256(path.read_bytes()).digest() == before
                    if path.suffix == '.docx':
                        with zipfile.ZipFile(destination) as archive:
                            assert archive.read('preserve.bin') == b'unchanged'
                            xml = archive.read('word/document.xml').decode()
                            assert re.search('[\u4e00-\u9fff]', xml) and 'Hello, world!' not in xml
                    elif path.suffix == '.srt':
                        assert '00:00:00,000 --> 00:00:02,000' in destination.read_text(encoding='utf-8')
                    elif path.suffix == '.pdf':
                        with pymupdf.open(destination) as document:
                            text = ''.join(page.get_text() for page in document)
                            assert re.search('[\u4e00-\u9fff]', text), text
                request({'op': 'file', 'source': 'en', 'target': 'zh', 'path': str(txt), 'output': str(txt)}, False)
                blank = folder / '无文字.pdf'
                with pymupdf.open() as document:
                    document.new_page(); document.save(blank)
                request({'op': 'file', 'source': 'en', 'target': 'zh', 'path': str(blank), 'output': str(blank.with_stem('不应生成'))}, False)
        finally:
            worker.stdin.close()
            try:
                worker.wait(timeout=20)
            except subprocess.TimeoutExpired:
                worker.kill(); worker.wait()
        assert worker.returncode == 0
    print('PASS: relocated engine, both directions, validation, TXT/SRT/DOCX/PDF, source preservation, EOF shutdown.')

if __name__ == '__main__':
    main()
