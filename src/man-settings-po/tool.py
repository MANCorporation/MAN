#!/usr/bin/env python3
"""Translations of MAN's own strings in GNOME Settings ("man-settings" domain).

    tool.py msgs                 print the strings (JSON list, the index is N below)
    tool.py import LANG FILE     FILE has "N|translation" lines (\\n escaped) -> LANG.po
    tool.py build LOCALEDIR DESKTOP-IN DESKTOP-OUT
                                 compile every LANG.po to LOCALEDIR/LANG/LC_MESSAGES/man-settings.mo
                                 and write the Updates panel desktop file with Name[LANG]/Comment[LANG]

Strings come from MAN's code in gcc-51: _() / N_() in cc-man-tweaks.c and
cc-man-updates-panel.c, cc_man_gettext() in the Apps and About panels, and
Name/Comment of the Updates panel desktop file. Pure Python, no gettext tools.
"""
import json, re, struct, sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
GCC = HERE.parent / 'gcc-51'
DOMAIN = 'man-settings'
SOURCES = [
    ('panels/common/cc-man-tweaks.c', r'\b(?:N_|_)\s*\('),
    ('panels/man-updates/cc-man-updates-panel.c', r'\b(?:N_|_)\s*\('),
    ('panels/applications/cc-applications-panel.c', r'\bcc_man_gettext\s*\('),
    ('panels/system/about/cc-about-page.c', r'\bcc_man_gettext\s*\('),
]
DESKTOP = 'panels/man-updates/gnome-man-updates-panel.desktop'
LIT = r'"((?:[^"\\]|\\.)*)"'


def c_unescape(s):
    return re.sub(r'\\(.)', lambda m: {'n': '\n', 't': '\t'}.get(m.group(1), m.group(1)), s)


def msgs():
    out = {}
    for rel, call in SOURCES:
        text = (GCC / rel).read_text(encoding='utf-8')
        for m in re.finditer(call + r'\s*((?:' + LIT + r'\s*)+)\)', text):
            s = ''.join(c_unescape(x) for x in re.findall(LIT, m.group(1)))
            out.setdefault(s, f'{rel}:{text.count(chr(10), 0, m.start()) + 1}')
    for line in (GCC / DESKTOP).read_text(encoding='utf-8').splitlines():
        m = re.match(r'(Name|Comment)=(.*)', line)
        if m:
            out.setdefault(m.group(2), DESKTOP)
    return out


def po_quote(s):
    s = s.replace('\\', '\\\\').replace('"', '\\"').replace('\t', '\\t')
    if '\n' in s:
        parts = s.split('\n')
        lines = [p + '\\n' for p in parts[:-1]] + ([parts[-1]] if parts[-1] else [])
        return '""\n' + '\n'.join(f'"{x}"' for x in lines)
    return f'"{s}"'


def write_po(path, messages, tr, lang):
    out = ['msgid ""', 'msgstr ""', f'"Project-Id-Version: {DOMAIN}\\n"', f'"Language: {lang}\\n"',
           '"MIME-Version: 1.0\\n"', '"Content-Type: text/plain; charset=UTF-8\\n"', '"Content-Transfer-Encoding: 8bit\\n"', '']
    for msg, where in messages.items():
        out += [f'#: {where}', 'msgid ' + po_quote(msg), 'msgstr ' + po_quote(tr.get(msg, '')), '']
    Path(path).write_text('\n'.join(out), encoding='utf-8')


def read_po(path):
    out, cur, field = {}, {}, None
    unq = lambda s: c_unescape(s[1:-1])
    def flush():
        if cur.get('msgid') and cur.get('msgstr'):
            out[cur['msgid']] = cur['msgstr']
    for line in Path(path).read_text(encoding='utf-8').splitlines() + ['']:
        line = line.strip()
        if line.startswith('msgid '):
            flush(); cur, field = {'msgid': unq(line[6:])}, 'msgid'
        elif line.startswith('msgstr '):
            cur['msgstr'], field = unq(line[7:]), 'msgstr'
        elif line.startswith('"') and field:
            cur[field] += unq(line)
        elif not line:
            flush(); cur, field = {}, None
    return out


def write_mo(path, catalog):
    entries = sorted(catalog.items())
    entries.insert(0, ('', 'Content-Type: text/plain; charset=UTF-8\n'))
    keys, vals = [k.encode() for k, _ in entries], [v.encode() for _, v in entries]
    n = len(entries)
    otab, ttab = 28, 28 + n * 8
    pos, offsets, blob = ttab + n * 8, [], b''
    for s in keys + vals:
        offsets.append((len(s), pos + len(blob)))
        blob += s + b'\0'
    head = struct.pack('<7I', 0x950412de, 0, n, otab, ttab, 0, 0)
    Path(path).parent.mkdir(parents=True, exist_ok=True)
    Path(path).write_bytes(head + b''.join(struct.pack('<2I', l, o) for l, o in offsets) + blob)


def import_tx(lang, path):
    messages = msgs()
    ids = list(messages)
    tx, errors = {}, []
    for ln in Path(path).read_text(encoding='utf-8').splitlines():
        if not ln.strip():
            continue
        n, _, t = ln.partition('|')
        if not n.isdigit():
            errors.append(f'bad line: {ln[:60]}'); continue
        tx[int(n)] = t.replace('\\n', '\n')
    errors += [f'missing: {i}' for i in range(len(ids)) if i not in tx]
    out = {}
    fmt = lambda s: re.findall(r'%[a-z]', s)
    for i, t in tx.items():
        if i >= len(ids):
            errors.append(f'{i}: out of range'); continue
        src = ids[i]
        if fmt(src) != fmt(t):
            errors.append(f'{i}: formats {fmt(src)} vs {fmt(t)}: {t}')
        if ('_' in src) != ('_' in t) and src.count('_') == 1:
            errors.append(f'{i}: mnemonic underscore: {t}')
        if not t.strip():
            errors.append(f'{i}: empty')
        out[src] = t
    if errors:
        print('\n'.join(errors)); sys.exit(1)
    write_po(HERE / f'{lang}.po', messages, out, lang)
    print(f'{lang}: {len(out)} ok')


def build(localedir, desktop_in, desktop_out):
    catalogs = {po.stem: read_po(po) for po in sorted(HERE.glob('*.po'))}
    for lang, cat in catalogs.items():
        write_mo(Path(localedir) / lang / 'LC_MESSAGES' / f'{DOMAIN}.mo', cat)
    out = []
    for line in Path(desktop_in).read_text(encoding='utf-8').splitlines():
        if re.match(r'(Name|Comment)\[', line):
            continue
        out.append(line)
        m = re.match(r'(Name|Comment)=(.*)', line)
        if m:
            out += [f'{m.group(1)}[{lang}]={cat[m.group(2)]}' for lang, cat in catalogs.items() if m.group(2) in cat]
    Path(desktop_out).write_text('\n'.join(out) + '\n', encoding='utf-8')
    print(f'{len(catalogs)} languages', file=sys.stderr)


if __name__ == '__main__':
    cmd = sys.argv[1] if len(sys.argv) > 1 else ''
    if cmd == 'msgs':
        m = msgs()
        write_po(HERE / f'{DOMAIN}.pot', m, {}, '')
        json.dump(list(m), sys.stdout, ensure_ascii=False, indent=0)
        print(f'\n{len(m)} messages', file=sys.stderr)
    elif cmd == 'import':
        import_tx(sys.argv[2], sys.argv[3])
    elif cmd == 'build':
        build(*sys.argv[2:5])
    else:
        print(__doc__); sys.exit(2)
