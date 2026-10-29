"""Stage pinned model data for isolated desktop checks. No user-text IO or network."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import zipfile

ROOT = Path(__file__).resolve().parent.parent
CACHE = ROOT / '.cache/model-downloads'


def fingerprint(path):
    return hashlib.file_digest(path.open('rb'), 'sha256').hexdigest()


def write_checked(destination, expected, source=None, data=None):
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        assert destination.stat().st_size == expected['size'] and fingerprint(destination) == expected['sha256'], f'Refusing to overwrite differing fixture: {destination.name}'
        return
    if source is not None:
        assert source.stat().st_size == expected['size'] and fingerprint(source) == expected['sha256']
        shutil.copyfile(source, destination)
    else:
        assert len(data) == expected['size'] and hashlib.sha256(data).hexdigest() == expected['sha256']
        destination.write_bytes(data)
    assert destination.stat().st_size == expected['size'] and fingerprint(destination) == expected['sha256']


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('destination', type=Path)
    args = parser.parse_args()
    destination = args.destination.resolve()
    assert destination.name == 'models' and any(destination.is_relative_to(ROOT / parent) for parent in ['.cache','release']), 'Fixtures must stay in a project cache/release models directory'
    catalog = json.loads((ROOT / 'src-tauri/model-catalog.json').read_text(encoding='utf-8'))
    for model in catalog:
        package = destination / 'packages' / model['folder']
        embedded = {file['path']: file['content'].encode('utf-8') for file in model.get('embedded_files', [])}
        if model.get('kind') == 'm2m100':
            for file in model['files']:
                target = (package / file['path']).resolve()
                assert target.is_relative_to(destination)
                if file['path'] in embedded: write_checked(target, file, data=embedded[file['path']])
                else: write_checked(target, file, source=CACHE / model['folder'] / file['path'])
        else:
            archive = CACHE / model['archive']['url'].rsplit('/',1)[-1]
            assert fingerprint(archive) == model['archive']['sha256']
            with zipfile.ZipFile(archive) as zip:
                for file in model['files']:
                    target = (package / file['path']).resolve()
                    assert target.is_relative_to(destination)
                    write_checked(target, file, data=zip.read(model['folder']+'/'+file['path']))
        for sbd in [model['segmentation'], *model.get('additional_segmentation', [])]:
            target=(destination / 'minisbd' / sbd['filename']).resolve()
            assert target.is_relative_to(destination)
            write_checked(target, sbd, source=CACHE / sbd['filename'])
        print('Staged verified model:', model['id'])
