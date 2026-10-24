# import.py LANG FILE: FILE has lines "N|translation" (\n escaped); validate
# against msgs.json and write man-installer/po/LANG.po.
import json, re, sys
sys.path.insert(0, '/home/kristihack/Documents/MANRUST/man-installer/po')
from extract import messages, write_po
lang, path = sys.argv[1], sys.argv[2]
msgs = messages()
ids = list(msgs)
tx, errors = {}, []
for ln in open(path, encoding='utf-8'):
    ln = ln.rstrip('\n')
    if not ln.strip():
        continue
    n, _, t = ln.partition('|')
    try:
        i = int(n)
    except ValueError:
        errors.append(f'bad line: {ln[:60]}'); continue
    if i in tx: errors.append(f'{i}: duplicate')
    tx[i] = t.replace('\\n', '\n')
missing = [i for i in range(len(ids)) if i not in tx]
if missing: errors.append(f'missing: {missing}')
out = {}
for i, t in tx.items():
    if i >= len(ids): errors.append(f'{i}: out of range'); continue
    src = ids[i]
    ph = lambda s: sorted(re.findall(r'\{[a-z_]+\}', s))
    if ph(src) != ph(t): errors.append(f'{i}: placeholders {ph(src)} vs {ph(t)}: {t}')
    if src.count('\n') != t.count('\n'): errors.append(f'{i}: line breaks differ')
    if src.startswith(' ') != t.startswith(' ') and not t.startswith('（'): errors.append(f'{i}: leading space')
    if not t.strip(): errors.append(f'{i}: empty')
    out[src] = t
if errors:
    print('\n'.join(errors)); sys.exit(1)
write_po(f'/home/kristihack/Documents/MANRUST/man-installer/po/{lang}.po', msgs, out, lang)
print(f'{lang}: {len(out)} ok')
