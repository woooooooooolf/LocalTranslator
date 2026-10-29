"""Smoke-test the final executable without Python/runtime or external model data."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('exe', type=Path)
args = parser.parse_args()
source = args.exe.resolve()
root = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='release-smoke-', dir=root / '.cache') as directory:
    directory = Path(directory)
    exe = directory / source.name
    shutil.copyfile(source, exe)
    env = {**os.environ, 'PATH': str(Path(os.environ['SystemRoot']) / 'System32')}
    for variable in ['PYTHONHOME', 'PYTHONPATH', 'ORT_DYLIB_PATH']:
        env.pop(variable, None)
    forbidden_output = directory / 'must-not-exist.txt'
    requests = [{'op': 'info'}, {'op': 'text', 'text': 'Release smoke test', 'source': 'zh', 'target': 'en'},
                {'op': 'file', 'path': str(exe), 'output': str(forbidden_output), 'source': 'en', 'target': 'zh'}]
    result = subprocess.run([str(exe), '--stdio'], input=''.join(json.dumps(p) + '\n' for p in requests),
                            text=True, encoding='utf-8', capture_output=True, timeout=30, env=env, cwd=root)
    assert result.returncode == 0, result.returncode
    assert not result.stderr.strip(), result.stderr[:300]
    replies = [json.loads(line) for line in result.stdout.splitlines()]
    assert len(replies) == 3
    assert replies[0]['ok'] and replies[0]['data']['native'] and not replies[0]['data']['packages']
    assert not replies[1]['ok'] and not replies[2]['ok'] and not forbidden_output.exists()
    assert not any(p.suffix.lower() in {'.dll', '.pyd', '.py'} for p in directory.rglob('*'))
    assert not (directory / 'data').exists()
    assert {str(p.relative_to(directory)) for p in directory.rglob('*') if p.is_file()} == {exe.name, 'models\\.manager\\lock'}
print('PASS: standalone EXE, no models/runtime extraction, no file export, empty stderr')
