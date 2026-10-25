# tag-sample-music.py DIR: give the MAN Sample Music MP3s an ID3v2.4 tag
# (title from the file name, artist kristi/VIS, album MAN Sample Music,
# track number, Artwork.jpg as front cover). v2.4 keeps "kristi/VIS" as one
# artist ("/" separates artists in v2.3). An existing ID3v2 tag is replaced.
import sys
from pathlib import Path

ARTIST, ALBUM = 'kristi/VIS', 'MAN Sample Music'


def syncsafe(n):
    return bytes([(n >> 21) & 0x7f, (n >> 14) & 0x7f, (n >> 7) & 0x7f, n & 0x7f])


def frame(fid, body):
    return fid.encode() + syncsafe(len(body)) + b'\0\0' + body


def text(fid, value):
    return frame(fid, b'\x03' + value.encode('utf-8'))


def strip_tag(data):
    if data[:3] == b'ID3':
        size = (data[6] << 21) | (data[7] << 14) | (data[8] << 7) | data[9]
        return data[10 + size + (10 if data[5] & 0x10 else 0):]
    return data


d = Path(sys.argv[1])
cover = (d / 'Artwork.jpg').read_bytes()
songs = sorted(p for p in d.glob('*.mp3') if not p.name.startswith('._'))
for n, p in enumerate(songs, 1):
    frames = b''.join([
        text('TIT2', p.stem), text('TPE1', ARTIST), text('TPE2', ARTIST), text('TALB', ALBUM),
        text('TRCK', f'{n}/{len(songs)}'),
        frame('APIC', b'\x03' + b'image/jpeg\0' + b'\x03' + b'\0' + cover),
    ])
    audio = strip_tag(p.read_bytes())
    p.write_bytes(b'ID3\x04\x00\x00' + syncsafe(len(frames)) + frames + audio)
    print(f'{n}. {p.stem}  — {ARTIST}, {ALBUM}')
