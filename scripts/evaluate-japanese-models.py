"""Maintainer-only CPU comparison with synthetic dialogue; never used by the app."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
import shutil
from pathlib import Path
import statistics
import subprocess
import sys
import time
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / '.cache/japanese-evaluation'
M2M_REPO = 'auralmira/m2m100-418M-ct2-int8'
M2M_REVISION = 'e205afefce2fd6933a1dca3b92b5063894f2f2bb'
LARGE_REPO = 'theDev997/m2m100-1.2B-ct2-int8'
LARGE_REVISION = 'cedd592d23478a20e87645851003025498df60ff'
M2M_FILES = ['config.json', 'model.bin', 'shared_vocabulary.json', 'sentencepiece.bpe.model', 'README.md']
SAMPLES = [
    ('ja', 'zh', 'お前のこと、ずっと好きだった。', '一直喜欢你；不能变成讨厌'),
    ('ja', 'zh', '別に、あんたのためじゃないんだからね！', '才不是为了你；保留否定与语气'),
    ('ja', 'zh', '昨日は助けてくれてありがとう。', '感谢昨天帮助'),
    ('ja', 'zh', '待って！まだ話は終わってない！', '等等，话还没有说完'),
    ('ja', 'zh', 'ここで会ったことは、二人だけの秘密ね。', '在这里见面的事是两个人的秘密'),
    ('ja', 'zh', '妹じゃなくて、姉です。', '不是妹妹，是姐姐'),
    ('ja', 'zh', '田中さんは来ないって。先に行こう。', '田中说不来了；我们先走'),
    ('ja', 'zh', 'そんなわけないだろ！', '怎么可能；否定反问'),
    ('ja', 'zh', 'ドキドキして、うまく話せない。', '紧张/心跳，没法好好说话'),
    ('ja', 'zh', '鍵は机の上に置いておいたよ。', '钥匙已放桌上'),
    ('ja', 'zh', '彼女は彼を助けた。彼は彼女にお礼を言った。', '她帮助他，他向她道谢；代词角色'),
    ('ja', 'zh', '「大丈夫？」\n「うん、少し休めば平気。」', '两行对话；休息后没事'),
    ('zh', 'ja', '我一直很喜欢你。', '喜欢你；过去至今'),
    ('zh', 'ja', '才不是特意为你准备的呢！', '傲娇否定'),
    ('zh', 'ja', '谢谢你昨天帮了我。', '昨天帮助与感谢'),
    ('zh', 'ja', '等等，我的话还没说完！', '等一下，尚未说完'),
    ('zh', 'ja', '今天见面的事，要对其他人保密哦。', '对别人保密'),
    ('zh', 'ja', '不是妹妹，是姐姐。', '姉，不是妹'),
    ('zh', 'ja', '田中说他不来了，我们先走吧。', '田中不来，我们先走'),
    ('zh', 'ja', '怎么可能发生这种事！', '反问否定'),
    ('zh', 'ja', '我紧张得心怦怦跳，说不出话来。', '紧张与说不出话'),
    ('zh', 'ja', '我把钥匙放在桌子上了。', '钥匙放在桌上'),
    ('zh', 'ja', '她救了他，他向她道谢。', '人物与代词方向'),
    ('zh', 'ja', '“你没事吧？”\n“嗯，休息一下就好了。”', '换行不丢失'),
]


def fetch(url, destination, size=None, digest=None):
    destination.parent.mkdir(parents=True, exist_ok=True)
    if not destination.exists():
        part = destination.with_suffix(destination.suffix + '.part')
        request = urllib.request.Request(url, headers={'User-Agent': 'Jingyi/1.0.0'})
        with urllib.request.urlopen(request, timeout=60) as response, part.open('wb') as output:
            count = 0
            while data := response.read(256 * 1024):
                output.write(data)
                count += len(data)
                if count // (32 * 1024 * 1024) != (count - len(data)) // (32 * 1024 * 1024):
                    print(f'{destination.name}: {count // (1024*1024)} MiB', flush=True)
        part.replace(destination)
    actual = hashlib.file_digest(destination.open('rb'), 'sha256').hexdigest()
    if size is not None and destination.stat().st_size != size:
        raise ValueError(f'Wrong size: {destination.name}')
    if digest is not None and actual != digest:
        raise ValueError(f'Wrong digest: {destination.name}')
    return {'size': destination.stat().st_size, 'sha256': actual, 'url': url}


def download(candidate='m2m100'):
    repo, revision = (LARGE_REPO, LARGE_REVISION) if candidate == 'm2m100-1.2b' else (M2M_REPO, M2M_REVISION)
    request = urllib.request.Request(f'https://huggingface.co/api/models/{repo}/revision/{revision}?blobs=true', headers={'User-Agent':'LocalTranslator/1.0'})
    metadata = json.load(urllib.request.urlopen(request, timeout=30))
    files = {item['rfilename']: item for item in metadata['siblings']}
    jobs = []
    for name in M2M_FILES:
        item = files[name]
        jobs.append((f'https://huggingface.co/{repo}/resolve/{revision}/{name}?download=true', CACHE/candidate/name,
                     item['size'], item.get('lfs', {}).get('sha256')))
    for pair in ([] if candidate == 'm2m100-1.2b' else ['ja_en', 'en_ja']):
        name = f'translate-{pair}-1_1.argosmodel'
        jobs.append((f'https://argos-net.com/v1/{name}', CACHE/'downloads'/name, None, None))
    jobs.append(('https://github.com/LibreTranslate/MiniSBD/releases/download/v0.0.1/ja.onnx', CACHE/'ja.onnx',None,None))
    with ThreadPoolExecutor(max_workers=4) as pool:
        results = list(pool.map(lambda job: fetch(*job), jobs))
    (CACHE/f'{candidate}-download-manifest.json').write_text(json.dumps(results,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    for pair in ([] if candidate == 'm2m100-1.2b' else ['ja_en','en_ja']):
        archive = CACHE/'downloads'/f'translate-{pair}-1_1.argosmodel'
        prefix = f'{pair}/'
        with zipfile.ZipFile(archive) as package:
            for item in package.infolist():
                if not item.filename.startswith(prefix) or '..' in item.filename.split('/') or '\\' in item.filename:
                    raise ValueError('Unexpected Argos archive path')
                relative = item.filename[len(prefix):]
                if item.is_dir() or not (relative.startswith('model/') or relative in ['metadata.json','sentencepiece.model','README.md']):
                    continue
                output = CACHE/f'argos-{pair}'/relative
                output.parent.mkdir(parents=True,exist_ok=True)
                output.write_bytes(package.read(item))
    print('Candidate weights downloaded and checked',flush=True)


def memory(handle=None):
    class Counters(ctypes.Structure):
        _fields_=[('cb',wintypes.DWORD),('faults',wintypes.DWORD)]+[(key,ctypes.c_size_t) for key in
            ['peak_ws','ws','peak_paged','paged','peak_nonpaged','nonpaged','pagefile','peak_pagefile','private']]
    kernel=ctypes.WinDLL('kernel32',use_last_error=True)
    kernel.GetCurrentProcess.restype=wintypes.HANDLE
    psapi=ctypes.WinDLL('psapi',use_last_error=True)
    psapi.GetProcessMemoryInfo.argtypes=[wintypes.HANDLE,ctypes.POINTER(Counters),wintypes.DWORD]
    counters=Counters(); counters.cb=ctypes.sizeof(counters)
    if not psapi.GetProcessMemoryInfo(handle or kernel.GetCurrentProcess(),ctypes.byref(counters),counters.cb):
        raise ctypes.WinError(ctypes.get_last_error())
    return {'peak_working_set_mib':round(counters.peak_ws/1024/1024,1),'private_mib':round(counters.private/1024/1024,1)}


def benchmark(candidate):
    import ctranslate2
    import sentencepiece as spm
    started=time.perf_counter()
    if candidate.startswith('m2m100'):
        translator=ctranslate2.Translator(str(CACHE/candidate),device='cpu',compute_type='int8',inter_threads=1,intra_threads=4)
        tokenizer=spm.SentencePieceProcessor(model_file=str(CACHE/candidate/'sentencepiece.bpe.model'))
        def translate(text,source,target):
            results=[]
            for paragraph in text.split('\n'):
                if not paragraph.strip(): results.append(paragraph);continue
                tokens=[f'__{source}__',*tokenizer.encode(paragraph,out_type=str),'</s>']
                output=translator.translate_batch([tokens],target_prefix=[[f'__{target}__']],beam_size=4,length_penalty=1.0,max_decoding_length=256)[0].hypotheses[0]
                results.append(tokenizer.decode(output[1:]))
            return '\n'.join(results)
    else:
        translators={};tokenizers={}
        for source,target in [('ja','en'),('en','ja'),('en','zh'),('zh','en')]:
            package = CACHE/f'argos-{source}_{target}' if 'ja' in [source,target] else ROOT/'models/packages'/f'translate-{source}_{target}-1_9'
            translators[source,target]=ctranslate2.Translator(str(package/'model'),device='cpu',compute_type='int8',inter_threads=1,intra_threads=4)
            tokenizers[source,target]=spm.SentencePieceProcessor(model_file=str(package/'sentencepiece.model'))
        def stage(text,source,target):
            tokenizer=tokenizers[source,target]
            tokens=tokenizer.encode(text,out_type=str)
            output=translators[source,target].translate_batch([tokens],beam_size=4,length_penalty=0.2,replace_unknowns=True,max_decoding_length=256)[0].hypotheses[0]
            return tokenizer.decode(output).replace('\u2581',' ').replace('_',' ').lstrip(' ')
        def translate(text,source,target):
            return '\n'.join(stage(stage(paragraph,source,'en'),'en',target) if paragraph.strip() else paragraph for paragraph in text.split('\n'))
    cold=time.perf_counter()-started
    translate('こんにちは。','ja','zh'); translate('你好。','zh','ja')
    records=[]
    for source,target,text,meaning in SAMPLES:
        start=time.perf_counter();result=translate(text,source,target);elapsed=time.perf_counter()-start
        records.append({'source':source,'target':target,'input':text,'output':result,'meaning':meaning,'seconds':round(elapsed,3)})
        print(json.dumps(records[-1],ensure_ascii=False),flush=True)
    report={'candidate':candidate,'threads':4,'ct2':ctranslate2.__version__,'cold_load_seconds':round(cold,3),**memory(),
        'p50_seconds':round(statistics.median(r['seconds'] for r in records),3),'samples':records}
    (CACHE/f'{candidate}-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:v for k,v in report.items() if k!='samples'},ensure_ascii=False),flush=True)


def native_benchmark(candidate, desktop_exe=None):
    import sentencepiece as spm
    from minisbd import SBDetect
    if desktop_exe is None:
        model_root=CACHE/f'native-{candidate}'
        package=model_root/'packages/m2m100-ja-zh'
        (package/'model').mkdir(parents=True,exist_ok=True)
        (model_root/'minisbd').mkdir(exist_ok=True)
        for name in ['model.bin','config.json','shared_vocabulary.json']:
            shutil.copyfile(CACHE/candidate/name,package/'model'/name)
        shutil.copyfile(CACHE/candidate/'sentencepiece.bpe.model',package/'sentencepiece.model')
        shutil.copyfile(CACHE/candidate/'README.md',package/'README.md')
        (package/'metadata.json').write_text(json.dumps({'from_code':'ja','to_code':'zh','from_name':'Japanese','to_name':'Chinese',
            'package_version':'1.0','model_type':'m2m100','bidirectional':True,'target_prefix':'__zh__'}),encoding='utf-8')
        shutil.copyfile(CACHE/'ja.onnx',model_root/'minisbd/ja.onnx')
        shutil.copyfile(ROOT/'models/minisbd/zh-hans.onnx',model_root/'minisbd/zh-hans.onnx')
        exe=ROOT/'crates/engine/target/release/check-engine.exe'
        command=[str(exe),str(model_root)]
    else:
        exe=desktop_exe.resolve()
        model_root=exe.parent/'models'
        entry=next(m for m in json.loads((ROOT/'src-tauri/model-catalog.json').read_text(encoding='utf-8')) if m['id']=='ja-zh')
        package=model_root/'packages'/entry['folder']
        command=[str(exe),'--stdio']
    child=subprocess.Popen(command,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding='utf-8')
    def request(payload):
        child.stdin.write(json.dumps(payload,ensure_ascii=False)+'\n');child.stdin.flush()
        line=child.stdout.readline()
        if not line: raise RuntimeError(f'Native stopped: {child.poll()}')
        response=json.loads(line)
        if not response['ok']: raise RuntimeError(response)
        return response
    try:
        info=request({'op':'info'})['data'];assert {('ja','zh'),('zh','ja')}.issubset({(p['from'],p['to']) for p in info['packages']}),info
        tokenizer=spm.SentencePieceProcessor(model_file=str(package/'sentencepiece.model'))
        detectors={code:SBDetect(str(model_root/'minisbd'/f'{language}.onnx'),use_gpu=False) for code,language in [('ja','ja'),('zh','zh-hans')]}
        parity=0
        for source,target,text,_ in SAMPLES:
            for paragraph in text.split('\n'):
                expected=[f'__{source}__',*tokenizer.encode(paragraph,out_type=str),'</s>']
                assert request({'op':'tokenize','text':paragraph,'source':source,'target':target})['data']['tokens']==expected
                actual=request({'op':'sentences','text':paragraph,'source':source,'target':target})['data']['sentences']
                assert actual==detectors[source].sentences(paragraph),(source,paragraph,actual)
                parity+=1
        # Both directions were used above, so subsequent timings are warm.
        request({'op':'text','text':'こんにちは。','source':'ja','target':'zh'})
        request({'op':'text','text':'你好。','source':'zh','target':'ja'})
        records=[]
        for source,target,text,meaning in SAMPLES:
            response=request({'op':'text','text':text,'source':source,'target':target})
            records.append({'source':source,'target':target,'input':text,'output':response['data']['text'],'meaning':meaning,'seconds':round(response['elapsed'],3)})
            print(json.dumps(records[-1],ensure_ascii=False),flush=True)
        report={'candidate':candidate,'backend':'native-Ruy','threads':4,'token_and_boundary_parity':parity,
            **memory(wintypes.HANDLE(int(child._handle))), 'p50_seconds':round(statistics.median(r['seconds'] for r in records),3),'samples':records}
        suffix='desktop-native' if desktop_exe is not None else 'native'
        (CACHE/f'{candidate}-{suffix}-report.json').write_text(json.dumps(report,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
        print(json.dumps({k:v for k,v in report.items() if k!='samples'},ensure_ascii=False),flush=True)
    finally:
        child.stdin.close();child.wait(timeout=30)
        assert child.returncode==0,child.returncode
        assert not child.stderr.read().strip(),'Unexpected native diagnostics'


if __name__=='__main__':
    parser=argparse.ArgumentParser();parser.add_argument('action',choices=['download','benchmark','native']);parser.add_argument('--candidate',choices=['m2m100','m2m100-1.2b','argos-pivot']);parser.add_argument('--exe',type=Path,help='Check a staged desktop EXE instead of the engine harness');args=parser.parse_args()
    if args.action=='download': download(args.candidate or 'm2m100')
    elif args.action=='native': native_benchmark(args.candidate or 'm2m100',args.exe)
    elif args.candidate: benchmark(args.candidate)
    else: parser.error('--candidate is required for benchmarking')
