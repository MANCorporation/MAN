#!/usr/bin/env python3
"""Collect the translatable messages of MAN's apps into po/man-installer.pot.

GUI: tr("…"), trf("…", …), n_("…"). System helper (its messages are
translated by the GUI when shown): step titles, fixed progress details and
fixed error messages. Also accent colour names and .desktop Name/Comment.
"""
import json, re, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LIT = r'"((?:[^"\\]|\\.)*)"'

def unescape(s):
    return re.sub(r'\\(.)', lambda m: {'n': '\n', 't': '\t'}.get(m.group(1), m.group(1)), s)

def messages():
    seen = {}
    def add(msg, where):
        if msg and re.search(r'[A-Za-z]', msg):
            seen.setdefault(msg, where)
    for f in sorted((ROOT / 'crates/man-gui/src').glob('*.rs')):
        src = f.read_text()
        for m in re.finditer(r'\b(?:tr|trf|n_)\(\s*' + LIT, src):
            add(unescape(m.group(1)), f'{f.name}:{src.count(chr(10), 0, m.start()) + 1}')
    for f in sorted((ROOT / 'crates/man-backend/src').glob('*.rs')):
        src = f.read_text()
        line = lambda pos: f'{f.name}:{src.count(chr(10), 0, pos) + 1}'
        for m in re.finditer(r'const STEPS[^=]*=\s*\[(.*?)\];', src, re.S):
            for s in re.finditer(LIT, m.group(1)):
                add(unescape(s.group(1)), line(m.start()))
        pats = [r'\bstep\([^,]*,[^,]*,\s*' + LIT, r'\bprogress\([^,]*,\s*' + LIT,
                r'\b(?:bail|anyhow)!\(\s*' + LIT + r'\s*\)', r'\bfail\(\s*' + LIT]
        for p in pats:
            for m in re.finditer(p, src):
                msg = unescape(m.group(1))
                # Lower-case ones are developer messages (man-mkupd, internals).
                if '{' not in msg and msg[:1].isupper():
                    add(msg, line(m.start()))
    common = (ROOT / 'crates/man-common/src/lib.rs').read_text()
    block = re.search(r'pub const ACCENTS[^=]*=\s*&\[(.*?)\];', common, re.S).group(1)
    for m in re.finditer(r'\("[a-z]+",\s*' + LIT, block):
        add(m.group(1), 'lib.rs:ACCENTS')
    for f in sorted((ROOT / 'data').rglob('*.desktop*')):
        for l in f.read_text().splitlines():
            m = re.match(r'(Name|Comment|GenericName)=(.*)', l)
            if m:
                add(m.group(2), f.name)
    return seen

def po_quote(s):
    s = s.replace('\\', '\\\\').replace('"', '\\"').replace('\t', '\\t')
    if '\n' in s:
        parts = s.split('\n')
        lines = [p + '\\n' for p in parts[:-1]] + ([parts[-1]] if parts[-1] else [])
        return '""\n' + '\n'.join(f'"{x}"' for x in lines)
    return f'"{s}"'

def write_po(path, msgs, translations=None, lang=''):
    out = ['msgid ""', 'msgstr ""', '"Project-Id-Version: man-installer\\n"', f'"Language: {lang}\\n"',
           '"MIME-Version: 1.0\\n"', '"Content-Type: text/plain; charset=UTF-8\\n"', '"Content-Transfer-Encoding: 8bit\\n"', '']
    for msg, where in msgs.items():
        out.append(f'#: {where}')
        out.append('msgid ' + po_quote(msg))
        out.append('msgstr ' + po_quote((translations or {}).get(msg, '')))
        out.append('')
    Path(path).write_text('\n'.join(out))

if __name__ == '__main__':
    msgs = messages()
    write_po(ROOT / 'po/man-installer.pot', msgs)
    if len(sys.argv) > 1 and sys.argv[1] == '--json':
        json.dump(list(msgs), sys.stdout, ensure_ascii=False, indent=0)
    print(f'{len(msgs)} messages', file=sys.stderr)
