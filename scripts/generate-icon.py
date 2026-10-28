"""Generate the geometric application icon without extra dependencies."""
import struct
import zlib
from pathlib import Path

size = 256
pixels = bytearray()
for y in range(size):
    for x in range(size):
        color = (38, 117, 101, 255)
        if ((42 <= x <= 148 and 53 <= y <= 165) and
                not (52 <= x <= 138 and 63 <= y <= 155)):
            color = (255, 255, 255, 255)
        if (64 <= x <= 122 and 85 <= y <= 93) or (89 <= x <= 97 and 73 <= y <= 136):
            color = (255, 255, 255, 255)
        if 112 <= x <= 217 and 97 <= y <= 205:
            color = (255, 255, 255, 255) if not (122 <= x <= 207 and 107 <= y <= 195) else (38, 117, 101, 255)
        if 124 <= y <= 179 and (abs(x - (164 - (y - 124) * .45)) < 4 or abs(x - (164 + (y - 124) * .45)) < 4):
            color = (255, 255, 255, 255)
        if 147 <= x <= 181 and 158 <= y <= 165:
            color = (255, 255, 255, 255)
        pixels.extend(color)
def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
raw = b''.join(b'\x00' + pixels[y*size*4:(y+1)*size*4] for y in range(size))
png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(raw)) + chunk(b'IEND', b'')
folder = Path(__file__).resolve().parent.parent / 'src-tauri/icons'
folder.mkdir(exist_ok=True)
(folder / 'icon.png').write_bytes(png)
(folder / 'icon.ico').write_bytes(struct.pack('<HHH', 0, 1, 1) + struct.pack('<BBBBHHII', 0, 0, 0, 0, 1, 32, len(png), 22) + png)
