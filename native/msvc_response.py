"""Build-only launcher: MSVC requires a BOM for non-ASCII response files."""
import codecs
from pathlib import Path
import subprocess
import sys

for argument in sys.argv[2:]:
    if argument.startswith('@'):
        path = Path(argument[1:].strip('"'))
        if path.is_file():
            data = path.read_bytes()
            if not data.startswith((codecs.BOM_UTF16_LE, codecs.BOM_UTF16_BE)):
                text = data.decode('utf-8-sig')
                path.write_bytes(codecs.BOM_UTF16_LE + text.encode('utf-16-le'))
raise SystemExit(subprocess.call(sys.argv[1:]))
