"""Real-model test: text/error requests must not write content to application data."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import uuid

ROOT = Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='worker-privacy-', dir=ROOT / 'data') as directory:
    session = Path(directory)
    marker = 'JYPRIVATE' + uuid.uuid4().hex
    worker = subprocess.Popen(
        [str(ROOT / 'runtime/py/python.exe'), '-u', '-B', '-X', 'utf8', str(ROOT / 'backend/worker.py')],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
        text=True, encoding='utf-8', cwd=ROOT,
        env={**os.environ, 'JINGYI_SESSION_DIR': str(session)},
    )
    def request(payload):
        worker.stdin.write(json.dumps(payload) + '\n'); worker.stdin.flush()
        response = json.loads(worker.stdout.readline())
        return response
    try:
        reply = request({'op': 'text', 'text': marker, 'source': 'en', 'target': 'zh'})
        assert reply['ok'], reply
        error = request({'op': marker})
        assert not error['ok']
        patterns = [value.encode(encoding) for value in [marker, reply['data']['text']]
                    for encoding in ['utf-8', 'utf-16le']]
        checked = 0
        for file in session.rglob('*'):
            if file.is_file():
                content = file.read_bytes()
                assert not any(pattern in content for pattern in patterns), file
                checked += 1
        print(f'PASS: real text and error requests left no input/output in session files ({checked} files scanned).')
        assert not list(session.rglob('*.log'))
    finally:
        worker.stdin.close()
        try:
            worker.wait(timeout=15)
        except subprocess.TimeoutExpired:
            worker.kill(); worker.wait()
    assert worker.returncode == 0
