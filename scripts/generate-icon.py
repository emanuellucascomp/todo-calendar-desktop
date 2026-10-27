"""Gera ícone original RGBA sem dependências externas (calendário com check)."""
import math
import struct
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def segment_distance(x, y, ax, ay, bx, by):
    dx, dy = bx - ax, by - ay
    position = max(0, min(1, ((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)))
    return math.hypot(x - ax - position * dx, y - ay - position * dy)


def render(size):
    pixels = bytearray()
    for y in range(size):
        for x in range(size):
            px, py = (x + .5) * 64 / size, (y + .5) * 64 / size
            inside = math.hypot(max(abs(px - 32) - 14, 0), max(abs(py - 34) - 12, 0)) <= 14
            rings = segment_distance(px, py, 20, 6, 20, 16) <= 3.5 or segment_distance(px, py, 44, 6, 44, 16) <= 3.5
            check = segment_distance(px, py, 20, 42, 28, 50) <= 3 or segment_distance(px, py, 28, 50, 45, 32) <= 3
            color = (26, 115, 232, 255) if inside or rings else (0, 0, 0, 0)
            if inside and 25 <= py <= 27:
                color = (118, 171, 241, 255)
            if inside and check:
                color = (255, 255, 255, 255)
            pixels.extend(color)
    return pixels


def png(size, pixels):
    def chunk(kind, data):
        return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data))
    rows = b''.join(b'\0' + pixels[row * size * 4:(row + 1) * size * 4] for row in range(size))
    return b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', size, size, 8, 6, 0, 0, 0)) + chunk(b'IDAT', zlib.compress(rows)) + chunk(b'IEND', b'')


def main():
    output = ROOT / 'src-tauri/icons'
    output.mkdir(parents=True, exist_ok=True)
    sizes = [32, 64, 128, 256]
    images = [png(size, render(size)) for size in sizes]
    offset = 6 + 16 * len(sizes)
    entries = []
    for size, data in zip(sizes, images):
        entries.append(struct.pack('<BBBBHHII', size % 256, size % 256, 0, 0, 1, 32, len(data), offset))
        offset += len(data)
    (output / 'icon.ico').write_bytes(struct.pack('<HHH', 0, 1, len(sizes)) + b''.join(entries) + b''.join(images))
    (output / 'icon.png').write_bytes(images[-1])
    print('Ícones originais gerados em src-tauri/icons.')


if __name__ == '__main__':
    main()
