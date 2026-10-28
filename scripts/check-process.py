"""Verify network refusal and unexpected-parent-exit cleanup without a server."""
from pathlib import Path
import os
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
PYTHON = ROOT / 'runtime/py/python.exe'
probe = '''
import io, runpy, socket, sys
sys.stdin = io.StringIO('')
runpy.run_path('backend/worker.py')
for operation in [lambda: socket.getaddrinfo('example.com', 443), lambda: socket.socket().connect(('127.0.0.1', 9))]:
    try:
        operation()
    except RuntimeError as error:
        assert '离线模式禁止网络连接' in str(error)
    else:
        raise AssertionError('Network access was not refused')
sys.__stdout__.write('PASS: DNS and socket connections refused before network I/O.\\n')
'''
subprocess.run([str(PYTHON), '-B', '-X', 'utf8', '-c', probe], cwd=ROOT, check=True, timeout=60)
if os.name == 'nt':
    parent = subprocess.Popen([str(PYTHON), '-c', 'import time; time.sleep(8)'])
    worker = subprocess.Popen([str(PYTHON), '-B', '-X', 'utf8', str(ROOT / 'backend/worker.py')],
                              env={**os.environ, 'JINGYI_PARENT_PID': str(parent.pid)},
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, cwd=ROOT)
    try:
        parent.wait(timeout=15)
        worker.wait(timeout=15)
        assert worker.returncode == 0
        print('PASS: worker exits when desktop parent disappears, even with stdin open.')
    finally:
        if parent.poll() is None:
            parent.kill(); parent.wait()
        if worker.poll() is None:
            worker.kill(); worker.wait()
