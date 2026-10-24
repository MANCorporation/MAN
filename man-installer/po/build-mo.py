#!/usr/bin/env python3
"""Compile po/*.po into gettext .mo files and translate .desktop files.

    build-mo.py LOCALEDIR [DESKTOP-FILE...]

Writes LOCALEDIR/<lang>/LC_MESSAGES/man-installer.mo for every po/<lang>.po
(no gettext tools needed), and adds Name[<lang>]= / Comment[<lang>]= lines
to the given .desktop files in place.
"""
import re, struct, sys
from pathlib import Path

PO_DIR = Path(__file__).resolve().parent
DOMAIN = 'man-installer'

def unquote(s):
    return re.sub(r'\\(.)', lambda m: {'n': '\n', 't': '\t'}.get(m.group(1), m.group(1)), s[1:-1])

def read_po(path):
    """{msgid: msgstr} of translated entries."""
    out, cur, field = {}, {}, None
    def flush():
        if cur.get('msgid') and cur.get('msgstr'):
            out[cur['msgid']] = cur['msgstr']
    for line in Path(path).read_text(encoding='utf-8').splitlines() + ['']:
        line = line.strip()
        if line.startswith('msgid '):
            flush(); cur, field = {'msgid': unquote(line[6:])}, 'msgid'
        elif line.startswith('msgstr '):
            cur['msgstr'], field = unquote(line[7:]), 'msgstr'
        elif line.startswith('"') and field:
            cur[field] += unquote(line)
        elif not line:
            flush(); cur, field = {}, None
    return out

def write_mo(path, catalog):
    entries = sorted(catalog.items())
    entries.insert(0, ('', 'Content-Type: text/plain; charset=UTF-8\n'))
    keys = [k.encode() for k, _ in entries]
    vals = [v.encode() for _, v in entries]
    n = len(entries)
    otab, ttab = 28, 28 + n * 8
    pos = ttab + n * 8
    offsets, blob = [], b''
    for s in keys + vals:
        offsets.append((len(s), pos + len(blob)))
        blob += s + b'\0'
    head = struct.pack('<7I', 0x950412de, 0, n, otab, ttab, 0, 0)
    table = b''.join(struct.pack('<2I', l, o) for l, o in offsets)
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_bytes(head + table + blob)

def main():
    localedir = Path(sys.argv[1])
    catalogs = {po.stem: read_po(po) for po in sorted(PO_DIR.glob('*.po'))}
    for lang, cat in catalogs.items():
        write_mo(localedir / lang / 'LC_MESSAGES' / f'{DOMAIN}.mo', cat)
    for desktop in sys.argv[2:]:
        lines = [l for l in Path(desktop).read_text().splitlines() if not re.match(r'(Name|Comment|GenericName)\[', l)]
        out = []
        for l in lines:
            out.append(l)
            m = re.match(r'(Name|Comment|GenericName)=(.*)', l)
            if m:
                for lang, cat in catalogs.items():
                    if m.group(2) in cat:
                        out.append(f'{m.group(1)}[{lang}]={cat[m.group(2)]}')
        Path(desktop).write_text('\n'.join(out) + '\n')
    print(f'{len(catalogs)} languages', file=sys.stderr)

if __name__ == '__main__':
    main()
