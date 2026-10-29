"""Maintainer-only: fetch official weights and record reviewed catalog hashes.

No runtime dependency. Re-run deliberately when updating pinned model versions.
"""
from concurrent.futures import ThreadPoolExecutor
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / '.cache/model-downloads'
CACHE.mkdir(parents=True, exist_ok=True)
FILES = ['metadata.json', 'README.md', 'sentencepiece.model',
         'model/config.json', 'model/model.bin', 'model/shared_vocabulary.json']


def fetch(url, filename=None, size=None, digest=None):
    path = CACHE / (filename or url.rsplit('/', 1)[-1])
    path.parent.mkdir(parents=True, exist_ok=True)
    if not path.exists():
        part = path.with_suffix(path.suffix + '.part')
        request = urllib.request.Request(url, headers={'User-Agent': 'Jingyi/0.4.0'})
        with urllib.request.urlopen(request, timeout=60) as response, part.open('wb') as out:
            while data := response.read(256 * 1024):
                out.write(data)
        part.replace(path)
    actual = hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()
    if size is not None: assert path.stat().st_size == size, ('Wrong size', path.name)
    if digest is not None: assert actual == digest, ('Wrong digest', path.name)
    return {'url': url, 'size': path.stat().st_size, 'sha256': actual}


def model(pair):
    source, target, language = pair
    folder = f'translate-{source}_{target}-1_9'
    archive = fetch(f'https://argos-net.com/v1/{folder}.argosmodel')
    sbd = fetch(f'https://github.com/LibreTranslate/MiniSBD/releases/download/v0.0.1/{language}.onnx')
    selected = []
    with zipfile.ZipFile(CACHE / (folder + '.argosmodel')) as package:
        assert package.testzip() is None
        for relative in FILES:
            data = package.read(folder + '/' + relative)
            existing = ROOT / 'models/packages' / folder / relative
            if existing.exists():
                assert data == existing.read_bytes(), ('Different existing weight', relative)
            selected.append({'path': relative, 'size': len(data),
                             'sha256': hashlib.sha256(data).hexdigest()})
    existing_sbd = ROOT / 'models/minisbd' / (language + '.onnx')
    if existing_sbd.exists():
        assert (CACHE / (language + '.onnx')).read_bytes() == existing_sbd.read_bytes()
    return {'id': f'{source}-{target}', 'source': source, 'target': target,
            'version': '1.9', 'folder': folder, 'archive': archive,
            'segmentation': {'filename': language + '.onnx', **sbd}, 'files': selected}


JAPANESE_PROFILES = {
    '418m': ('auralmira/m2m100-418M-ct2-int8','e205afefce2fd6933a1dca3b92b5063894f2f2bb','m2m100','418M','137949450f9cf691f3c92084f14123e82d05ecb23ac3cd688308ce1a95214572'),
    '1.2b': ('theDev997/m2m100-1.2B-ct2-int8','cedd592d23478a20e87645851003025498df60ff','m2m100-1.2b','1.2B','0d95242f9d0db65d8a795e9cabf91be9c31d751598cd478cf62b614e9942067b'),
}


def japanese_model(profile):
    repo, revision, evaluation_folder, parameters, weight_hash = JAPANESE_PROFILES[profile]
    request=urllib.request.Request(f'https://huggingface.co/api/models/{repo}/revision/{revision}?blobs=true',headers={'User-Agent':'LocalTranslator/1.0'})
    metadata=json.load(urllib.request.urlopen(request,timeout=30))
    assert metadata['sha']==revision
    remote={file['rfilename']:file for file in metadata['siblings']}
    assert remote['model.bin']['lfs']['sha256']==weight_hash
    assert remote['sentencepiece.bpe.model']['lfs']['sha256']=='d8f7c76ed2a5e0822be39f0a4f95a55eb19c78f4593ce609e2edbc2aea4d380a'
    folder=f'translate-ja_zh-m2m100-{profile}-int8'
    mappings={'model.bin':'model/model.bin','config.json':'model/config.json','shared_vocabulary.json':'model/shared_vocabulary.json','sentencepiece.bpe.model':'sentencepiece.model'}
    assets={}
    for name,relative in mappings.items():
        destination=CACHE/folder/relative
        source=ROOT/'.cache/japanese-evaluation'/evaluation_folder/name
        if not destination.exists() and source.exists():
            if 'lfs' in remote[name]:
                assert hashlib.file_digest(source.open('rb'),'sha256').hexdigest()==remote[name]['lfs']['sha256']
            else:
                data=source.read_bytes()
                assert hashlib.sha1(f'blob {len(data)}\0'.encode()+data).hexdigest()==remote[name]['blobId']
            destination.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(source,destination)
        asset=fetch(f'https://huggingface.co/{repo}/resolve/{revision}/{name}?download=true',f'{folder}/{relative}',remote[name]['size'],remote[name].get('lfs',{}).get('sha256'))
        data=destination.read_bytes() if name!='model.bin' else None
        if data is not None and 'lfs' not in remote[name]:
            assert hashlib.sha1(f'blob {len(data)}\0'.encode()+data).hexdigest()==remote[name]['blobId']
        assets[relative]=asset
    version=f'm2m100-{profile}-int8-{revision[:8]}'
    package_metadata=json.dumps({'from_code':'ja','to_code':'zh','from_name':'Japanese','to_name':'Chinese','package_version':version,
        'model_type':'m2m100','bidirectional':True,'target_prefix':'__zh__'},ensure_ascii=False,sort_keys=True)+'\n'
    license_text=(ROOT/'third_party/m2m100/LICENSE.txt').read_text(encoding='utf-8')
    readme=f'# Japanese ↔ Chinese: M2M100 {parameters} INT8\n\nBase model: https://huggingface.co/facebook/m2m100_{parameters}\nCommunity conversion: https://huggingface.co/{repo}\nPinned revision: {revision}\n\nThis bundle contains model data only. The community conversion is not an official Meta CTranslate2 release. Translation runs locally; quality is model-dependent.\n\n'+license_text
    embedded=[{'path':'metadata.json','content':package_metadata},{'path':'README.md','content':readme}]
    files=[{'path':path,'size':asset['size'],'sha256':asset['sha256']} for path,asset in assets.items()]
    files += [{'path':item['path'],'size':len(item['content'].encode()),'sha256':hashlib.sha256(item['content'].encode()).hexdigest()} for item in embedded]
    ja=fetch('https://github.com/LibreTranslate/MiniSBD/releases/download/v0.0.1/ja.onnx')
    zh=fetch('https://github.com/LibreTranslate/MiniSBD/releases/download/v0.0.1/zh-hans.onnx')
    return {'id':'ja-zh','source':'ja','target':'zh','version':version,'folder':folder,'kind':'m2m100','name':f'M2M100 {parameters} INT8','source_profile':profile,
        'bidirectional':True,'archive':assets['model/model.bin'],'segmentation':{'filename':'ja.onnx',**ja},
        'additional_segmentation':[{'filename':'zh-hans.onnx',**zh}], 'downloads':[{'path':path,**asset} for path,asset in assets.items() if path!='model/model.bin'],
        'embedded_files':embedded,'files':files}


def verify_catalog(catalog):
    def verify(entry):
        bundle=entry.get('kind')=='m2m100'
        if bundle:
            fetch(**dict(url=entry['archive']['url'],filename=f"{entry['folder']}/model/model.bin",size=entry['archive']['size'],digest=entry['archive']['sha256']))
            for remote in entry['downloads']:
                fetch(remote['url'],f"{entry['folder']}/{remote['path']}",remote['size'],remote['sha256'])
            contents={item['path']:item['content'].encode() for item in entry['embedded_files']}
            for file in entry['files']:
                if file['path'] in contents: data=contents[file['path']]
                else:
                    path=CACHE/entry['folder']/file['path']
                    assert path.stat().st_size==file['size'] and hashlib.file_digest(path.open('rb'),'sha256').hexdigest()==file['sha256']
                    continue
                assert len(data)==file['size'] and hashlib.sha256(data).hexdigest()==file['sha256']
        else:
            fetch(entry['archive']['url'],size=entry['archive']['size'],digest=entry['archive']['sha256'])
            with zipfile.ZipFile(CACHE/entry['archive']['url'].rsplit('/',1)[-1]) as archive:
                assert archive.testzip() is None
                for file in entry['files']:
                    data=archive.read(entry['folder']+'/'+file['path'])
                    assert len(data)==file['size'] and hashlib.sha256(data).hexdigest()==file['sha256']
        for segmentation in [entry['segmentation'],*entry.get('additional_segmentation',[])]:
            fetch(segmentation['url'],segmentation['filename'],segmentation['size'],segmentation['sha256'])
    with ThreadPoolExecutor(max_workers=3) as pool:
        list(pool.map(verify,catalog))
    print('PASS: pinned archives, weight files, metadata and sentence models match the committed catalog')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--verify', action='store_true', help='Verify committed catalog; never update it')
    parser.add_argument('--japanese',choices=JAPANESE_PROFILES,help='Explicitly select the reviewed Japanese bundle when generating')
    args = parser.parse_args()
    output = ROOT / 'src-tauri/model-catalog.json'
    if args.verify:
        verify_catalog(json.loads(output.read_text(encoding='utf-8')))
        raise SystemExit(0)
    with ThreadPoolExecutor(max_workers=2) as pool:
        catalog = list(pool.map(model, [('en', 'zh', 'en'), ('zh', 'en', 'zh-hans')]))
    profile=args.japanese
    if profile is None and output.exists():
        profile=next((item.get('source_profile') for item in json.loads(output.read_text(encoding='utf-8')) if item.get('kind')=='m2m100'),None)
    if profile: catalog.append(japanese_model(profile))
    output.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print('Generated pinned model catalog:',[(entry['id'],entry['version']) for entry in catalog])
