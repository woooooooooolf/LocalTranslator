"""Native integration and synthetic parity checks. Python is test tooling only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET
import zipfile

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser()
parser.add_argument('--exe', type=Path, default=ROOT/'crates/engine/target/release/check-engine.exe')
args = parser.parse_args()
native_exe = args.exe.resolve()
models = ROOT/'models' if native_exe.name == 'check-engine.exe' else native_exe.parent/'models'
model_fingerprints={str(path.relative_to(models)):hashlib.sha256(path.read_bytes()).hexdigest() for path in models.rglob('*') if path.is_file()}
command = [str(native_exe), str(models)] if native_exe.name == 'check-engine.exe' else [str(native_exe), '--stdio']
report = {'tokenization': 0, 'sentence_boundaries': 0, 'translation_exact': 0, 'translation_total': 0, 'differences': [], 'documents': []}

class Process:
    def __init__(self, command, environment=None, quiet=False):
        self.quiet=quiet
        self.child = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                      text=True, encoding='utf-8', cwd=ROOT/'.cache', env=environment)
    def request(self, payload, success=True):
        self.child.stdin.write(json.dumps(payload, ensure_ascii=False)+'\n'); self.child.stdin.flush()
        line=self.child.stdout.readline()
        if not line:
            raise AssertionError(('Process stopped',self.child.poll(),self.child.stderr.read()))
        result=json.loads(line)
        assert result['ok'] == success, result
        return result.get('data')
    def close(self):
        self.child.stdin.close()
        try: self.child.wait(timeout=20)
        except subprocess.TimeoutExpired: self.child.kill(); self.child.wait()
        assert self.child.returncode == 0, self.child.returncode
        diagnostics=self.child.stderr.read()
        if self.quiet:assert not diagnostics.strip(),('Unexpected native diagnostics',diagnostics[:500])

def sha(path): return hashlib.sha256(path.read_bytes()).digest()
with tempfile.TemporaryDirectory(prefix='native-check-',dir=ROOT/'.cache') as temp:
    temp=Path(temp)
    # Keep all reference-Python settings and temporary files within the test tree.
    for variable,relative in {'USERPROFILE':'home','APPDATA':'appdata','LOCALAPPDATA':'local',
        'XDG_CONFIG_HOME':'config','XDG_CACHE_HOME':'cache','TEMP':'temp','TMP':'temp'}.items():
        path=temp/relative;path.mkdir(exist_ok=True);os.environ[variable]=str(path)
    os.environ.update({'ARGOS_PACKAGES_DIR':str(ROOT/'runtime/packages'),
        'XDG_DATA_HOME':str(ROOT/'runtime/assets'),'ARGOS_DEVICE_TYPE':'cpu','ARGOS_CHUNK_TYPE':'MINISBD'})
    import sentencepiece
    from minisbd import SBDetect
    native_environment={**os.environ,'PATH':str(Path(os.environ['SystemRoot'])/'System32')}
    for key in ['PYTHONHOME','PYTHONPATH','ORT_DYLIB_PATH']:native_environment.pop(key,None)
    native=Process(command,native_environment,quiet=True)
    baseline=Process([str(ROOT/'runtime/py/python.exe'),'-u','-B','-X','utf8',str(ROOT/'backend/worker.py')],
                     {**os.environ,'JINGYI_SESSION_DIR':str(temp/'reference-session')})
    try:
        info=native.request({'op':'info'})
        assert info['native'] and 'pdf' not in info['formats'] and len(info['formats'])==8
        for source,target,language,samples in [
            ('en','zh','en',['Hello, world!','This translation runs locally on the CPU.',
             'Please check the USB connection.','Do not save this text automatically.',
             'A short sentence. Another sentence.','Dr. Smith arrived at 3.5 p.m. Is he here?',
             'Write to person@example.com. Visit https://example.org/help.','Unicode: café, ﬁ, ＡＢＣ, ①, 🙂.',
             '  Leading spaces and\ttabs.  ','First line.\n\nThird line.',
             'This is a sentence. This is another sentence. '*40]),
            ('zh','en','zh-hans',['你好，世界！','这个翻译工具在本地 CPU 上运行。',
             '请检查 USB 连接。','不要自动保存这段文字。','这是第一句话。这是第二句话。',
             '温度为23.5度，编号是A_001。','请访问 https://example.org/help。然后继续。',
             '特殊字符：全角ＡＢＣ，表情🙂，以及引号“你好”。','  前面有空格。  ',
             '第一行。\n\n第三行。','这是一个离线翻译工具。它使用本地模型处理文字。'*65])]:
            package=ROOT/'runtime/packages'/f'translate-{source}_{target}-1_9'
            tokenizer=sentencepiece.SentencePieceProcessor(model_file=str(package/'sentencepiece.model'))
            detector=SBDetect(str(ROOT/'runtime/assets/argos-translate/minisbd'/f'{language}.onnx'),use_gpu=False)
            for text in samples:
                tokens=native.request({'op':'tokenize','text':text,'source':source,'target':target})['tokens']
                assert tokens == tokenizer.encode(text,out_type=str),('tokenization',source,text)
                report['tokenization']+=1
                for paragraph in text.split('\n'):
                    if not paragraph.strip():continue
                    sentences=native.request({'op':'sentences','text':paragraph,'source':source,'target':target})['sentences']
                    expected=detector.sentences(paragraph)
                    assert sentences == expected,('sentence boundaries',source,paragraph[:100],sentences[:4],expected[:4])
                    report['sentence_boundaries']+=1
                payload={'op':'text','text':text,'source':source,'target':target}
                actual=native.request(payload)['text'];expected=baseline.request(payload)['text']
                assert actual.strip()
                report['translation_total']+=1
                if actual==expected:report['translation_exact']+=1
                else:report['differences'].append({'source':source,'input':text[:200], 'expected':expected[:300], 'actual':actual[:300]})
        native.request({'op':'text','source':'en','target':'zh','text':''},False)
        native.request({'op':'text','source':'en','target':'zh','text':'a'*20001},False)
        native.request({'op':'text','source':'en','target':'en','text':'Hello'},False)
        files=temp/'文档 空格';files.mkdir()
        inputs=[]
        txt=files/'文字.txt';txt.write_text('Hello, world!\nSecond line.',encoding='utf-8');inputs.append(txt)
        srt=files/'字幕.srt';srt.write_bytes(b'1\r\n00:00:00,000 --> 00:00:02,000\r\n<i>Hello, world!</i>\r\n\r\n');inputs.append(srt)
        html=files/'网页.html';html.write_text('<!doctype html><html><head><title>keep title</title></head><body><p>Hello <b>world!</b></p><script>keep_code()</script></body></html>',encoding='utf-8');inputs.append(html)
        for extension in ['docx','pptx','odt','odp','epub']:
            path=files/f'容器.{extension}'
            entries={
              'docx':('word/document.xml','<w:document xmlns:w="urn:word"><w:body><w:p><w:r><w:t>Hello </w:t></w:r><w:r><w:t>world!</w:t></w:r></w:p></w:body></w:document>'),
              'pptx':('ppt/slides/slide1.xml','<a:root xmlns:a="urn:slide"><a:p><a:r><a:t>Hello, world!</a:t></a:r></a:p></a:root>'),
              'odt':('content.xml','<root xmlns:text="urn:text"><text:p>Hello, world!</text:p></root>'),
              'odp':('content.xml','<root xmlns:text="urn:text"><text:p>Hello, world!</text:p></root>'),
              'epub':('chapter.xhtml','<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>keep title</title></head><body><p>Hello, world!</p><br/></body></html>'),
            }
            with zipfile.ZipFile(path,'w') as archive:
                if extension in ['odt','odp','epub']:archive.writestr('mimetype','application/epub+zip' if extension=='epub' else 'application/vnd.oasis.opendocument.text',compress_type=zipfile.ZIP_STORED)
                name,xml=entries[extension];archive.writestr(name,xml)
                archive.writestr('assets/keep.bin',b'UNCHANGED\x00\x01')
            inputs.append(path)
        for path in inputs:
            before=sha(path);output=path.with_stem(path.stem+'-译文')
            native.request({'op':'file','source':'en','target':'zh','path':str(path),'output':str(output)})
            assert output.is_file() and sha(path)==before and output.read_bytes()!=path.read_bytes()
            if path.suffix in ['.docx','.pptx','.odt','.odp','.epub']:
                with zipfile.ZipFile(output) as archive:
                    assert archive.read('assets/keep.bin') == b'UNCHANGED\x00\x01'
                    for name in archive.namelist():
                        if name.endswith(('.xml','.xhtml')):
                            data=archive.read(name);ET.fromstring(data)
                            if name != 'assets/keep.bin':assert re.search('[\u4e00-\u9fff]',data.decode('utf-8'))
                    if path.suffix=='.epub':assert archive.infolist()[0].filename=='mimetype' and archive.infolist()[0].compress_type==zipfile.ZIP_STORED
            elif path.suffix=='.srt':assert b'00:00:00,000 --> 00:00:02,000' in output.read_bytes() and b'<i>' in output.read_bytes()
            elif path.suffix=='.html':assert 'keep_code()' in output.read_text(encoding='utf-8') and 'keep title' in output.read_text(encoding='utf-8')
            report['documents'].append(path.suffix)
        native.request({'op':'file','source':'en','target':'zh','path':str(txt),'output':str(txt)},False)
        pdf=files/'removed.pdf';pdf.write_bytes(b'%PDF-test')
        native.request({'op':'file','source':'en','target':'zh','path':str(pdf),'output':str(files/'out.pdf')},False)
        assert not (files/'out.pdf').exists()
    finally:
        native.close();baseline.close()
assert model_fingerprints == {str(path.relative_to(models)):hashlib.sha256(path.read_bytes()).hexdigest() for path in models.rglob('*') if path.is_file()}, 'Model directory was modified'
(ROOT/'.cache/native-engine-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps({key:value for key,value in report.items() if key!='differences'},ensure_ascii=False))
print('Differences recorded:',len(report['differences']))
print('PASS: native tokenization, sentence boundaries, translation execution, 8 document paths and input guards.')
