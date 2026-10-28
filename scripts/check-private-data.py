"""Scan application data for an explicit synthetic UI audit marker, never user text."""
import argparse
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--directory', type=Path, required=True)
parser.add_argument('--marker', required=True)
args = parser.parse_args()
patterns = [args.marker.encode('utf-8'), args.marker.encode('utf-16le'), args.marker.encode('utf-16be')]
checked, unreadable, found = 0, [], []
for path in args.directory.rglob('*'):
    if not path.is_file():
        continue
    try:
        with path.open('rb') as stream:
            tail = b''
            while block := stream.read(1024 * 1024):
                data = tail + block
                if any(pattern in data for pattern in patterns):
                    found.append(str(path)); break
                tail = data[-max(map(len, patterns)):]
        checked += 1
    except OSError:
        unreadable.append(str(path))
print(f'Checked files: {checked}; unreadable: {len(unreadable)}; marker matches: {len(found)}')
for path in found:
    print('MATCH:', path)
if unreadable:
    print('Locked/unreadable files were not verified; repeat after closing the application.')
if found:
    raise SystemExit(1)
