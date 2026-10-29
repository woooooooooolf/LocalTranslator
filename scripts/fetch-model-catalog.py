"""Maintainer-only: fetch official weights and record reviewed catalog hashes.

No runtime dependency. Re-run deliberately when updating pinned model versions.
"""
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / '.cache/model-downloads'
CACHE.mkdir(parents=True, exist_ok=True)
FILES = ['metadata.json', 'README.md', 'sentencepiece.model',
         'model/config.json', 'model/model.bin', 'model/shared_vocabulary.json']


def fetch(url):
    path = CACHE / url.rsplit('/', 1)[-1]
    if not path.exists():
        part = path.with_suffix(path.suffix + '.part')
        request = urllib.request.Request(url, headers={'User-Agent': 'Jingyi/0.4.0'})
        with urllib.request.urlopen(request, timeout=60) as response, part.open('wb') as out:
            while data := response.read(256 * 1024):
                out.write(data)
        part.replace(path)
    return {'url': url, 'size': path.stat().st_size,
            'sha256': hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()}


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
            assert data == existing.read_bytes(), ('Different existing weight', relative)
            selected.append({'path': relative, 'size': len(data),
                             'sha256': hashlib.sha256(data).hexdigest()})
    assert (CACHE / (language + '.onnx')).read_bytes() == (ROOT / 'models/minisbd' / (language + '.onnx')).read_bytes()
    return {'id': f'{source}-{target}', 'source': source, 'target': target,
            'version': '1.9', 'folder': folder, 'archive': archive,
            'segmentation': {'filename': language + '.onnx', **sbd}, 'files': selected}


if __name__ == '__main__':
    with ThreadPoolExecutor(max_workers=2) as pool:
        catalog = list(pool.map(model, [('en', 'zh', 'en'), ('zh', 'en', 'zh-hans')]))
    output = ROOT / 'src-tauri/model-catalog.json'
    output.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + '\n', encoding='utf-8', newline='\n')
    print(json.dumps(catalog, ensure_ascii=False, indent=2))
