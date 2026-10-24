# man-installer 3.10.1: keep .deb apps across MAN updates. Rebuilds tx/LANG.txt
# in the new message order: old translations by message (msgs-3.10.0.json),
# new ones from tx3101.py (lists in KEYS order).   Then: import.py LANG tx/LANG.txt
import glob, json, os, re, sys
sys.path.insert(0, '/home/kristihack/Documents/MANRUST/man-installer/po')
from extract import messages
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from tx3101 import L as T

KEYS = [
"Keep After MAN Updates",
"MAN installs the app again after each update. Its package files are kept for that.",
"Apps from .deb packages become part of the system. A MAN update replaces the system, so they are only installed again after an update if you keep them (next steps).",
"Reinstalling kept apps",
]

ph = lambda s: sorted(re.findall(r'\{[a-z_]+\}', s))
bad = [f'{l}: {t!r}' for l, tr in T.items() for k, t in zip(KEYS, tr) if len(tr) != len(KEYS) or ph(k) != ph(t) or not t.strip()]
if bad or len(T) != 71:
    sys.exit('\n'.join(bad) or f'{len(T)} languages')
old_ids = json.load(open(os.path.join(HERE, 'msgs-3.10.0.json')))
ids = list(messages())
assert all(k in ids for k in KEYS), [k for k in KEYS if k not in ids]
json.dump(ids, open(os.path.join(HERE, 'msgs-3.10.1.json'), 'w'), ensure_ascii=False, indent=0)
for f in sorted(glob.glob(os.path.join(HERE, 'tx', '*.txt'))):
    lang = os.path.basename(f)[:-4]
    old = {}
    for ln in open(f, encoding='utf-8'):
        n, _, t = ln.rstrip('\n').partition('|')
        if n.isdigit():
            old[old_ids[int(n)]] = t
    new = dict(zip(KEYS, T[lang]))
    missing = [m for m in ids if m not in new and m not in old]
    if missing:
        sys.exit(f'{lang}: missing {missing}')
    open(f, 'w', encoding='utf-8').write('\n'.join(f'{i}|{new.get(m, old.get(m))}' for i, m in enumerate(ids)) + '\n')
print(len(T), 'languages,', len(ids), 'messages')
