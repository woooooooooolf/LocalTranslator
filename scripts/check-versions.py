"""Require a single release version across manifests, lockfiles, About and tags."""
import json
import os
from pathlib import Path
import re
import tomllib

root = Path(__file__).resolve().parent.parent
version = json.loads((root / 'package.json').read_text(encoding='utf-8'))['version']
assert re.fullmatch(r'\d+\.\d+\.\d+', version), 'Release version must be x.y.z'
lock = json.loads((root / 'package-lock.json').read_text(encoding='utf-8'))
assert lock['version'] == lock['packages']['']['version'] == version, 'npm lock version mismatch'
assert json.loads((root / 'src-tauri/tauri.conf.json').read_text(encoding='utf-8'))['version'] == version, 'Tauri version mismatch'
for directory, name in [('src-tauri', 'local-translate'), ('crates/engine', 'jingyi-engine')]:
    manifest = tomllib.loads((root / directory / 'Cargo.toml').read_text(encoding='utf-8'))
    assert manifest['package']['version'] == version, f'{directory} manifest version mismatch'
    packages = tomllib.loads((root / directory / 'Cargo.lock').read_text(encoding='utf-8'))['package']
    assert next(p for p in packages if p['name'] == name)['version'] == version, f'{directory} lock version mismatch'
interface = (root / 'src/main.ts').read_text(encoding='utf-8')
assert f'<small>{version}</small>' in interface and f'<span>v{version}</span>' in interface, 'About version mismatch'
if os.environ.get('GITHUB_REF_TYPE') == 'tag':
    assert os.environ.get('GITHUB_REF_NAME') == 'V' + version, 'Tag/version mismatch'
print(f'PASS: manifests, lockfiles, About and release tag agree on {version}')
