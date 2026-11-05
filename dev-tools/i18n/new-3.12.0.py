# man-installer 3.12.0: Windows programs. Rebuilds tx/LANG.txt in the new message
# order: old translations by message (msgs-3.10.1.json), new ones from tx312_*.py
# (lists in KEYS order).   Then: import.py LANG tx/LANG.txt
import glob, json, os, re, sys
sys.path.insert(0, '/home/kristihack/Documents/MANRUST/man-installer/po')
from extract import messages
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
T = {}
for part in 'abcdef':
    T.update(__import__(f'tx312_{part}').L)

KEYS = [
"Windows support could not be started.",
"Starting the Windows Program",
"The first start takes a moment.",
"The Windows Program Could Not Start",
"Running Windows programs (.exe) is not recommended, as they can likely contain malware or things that can harm your privacy.",
"MAN could not check this program for malware: the malware database is not ready yet. Connect to the internet and try again in a few minutes.",
"Run This Windows Program?",
"_Run It",
"Run a Program Flagged as Malware?",
"This is your last chance to stop. MAN found malware in this program, and Windows programs are not isolated from your files. It could:\n\n• steal your passwords, documents and photos\n• delete your files or hold them for ransom\n• spy on what you do and keep running in the background\n\nMAN cannot undo what the program does. Only continue if you know exactly why this program was flagged.",
"Run _Anyway",
"MAN flagged this Windows program (.exe) as malware, which can harm your data or privacy. It should not be run and should be moved to the trash.",
"Detected as: {name}",
"MAN Flagged This Program as Malware",
"_Move to Trash",
"_Bypass and Run",
"Windows Program",
"This Is Not a Windows Program",
"This file cannot be run.",
"This Program Cannot Run on This Computer",
"It was made for {arch} processors.",
"Checking This Program for Malware",
"Check a Windows program for malware and run it",
]

ph = lambda s: sorted(re.findall(r'\{[a-z_]+\}', s))
ACCEL = {7, 10, 14, 15}
bad = []
for l, tr in T.items():
    if len(tr) != len(KEYS):
        bad.append(f'{l}: {len(tr)} strings')
        continue
    for i, (k, t) in enumerate(zip(KEYS, tr)):
        if ph(k) != ph(t) or not t.strip() or k.count('\n') != t.count('\n') or k.count('•') != t.count('•'):
            bad.append(f'{l} {i}: {t[:70]!r}')
        if i in ACCEL and '_' not in t:
            bad.append(f'{l} {i}: no accelerator: {t!r}')
if bad or len(T) != 71:
    sys.exit('\n'.join(bad) or f'{len(T)} languages')
old_ids = json.load(open(os.path.join(HERE, 'msgs-3.10.1.json')))
ids = list(messages())
assert all(k in ids for k in KEYS), [k for k in KEYS if k not in ids]
json.dump(ids, open(os.path.join(HERE, 'msgs-3.12.0.json'), 'w'), ensure_ascii=False, indent=0)
for f in sorted(glob.glob(os.path.join(HERE, 'tx', '*.txt'))):
    lang = os.path.basename(f)[:-4]
    old = {}
    for ln in open(f, encoding='utf-8'):
        n, _, t = ln.rstrip('\n').partition('|')
        if n.isdigit():
            old[old_ids[int(n)]] = t
    new = {k: v.replace('\n', '\\n') for k, v in zip(KEYS, T[lang])}
    missing = [m for m in ids if m not in new and m not in old]
    if missing:
        sys.exit(f'{lang}: missing {missing}')
    open(f, 'w', encoding='utf-8').write('\n'.join(f'{i}|{new.get(m, old.get(m))}' for i, m in enumerate(ids)) + '\n')
print(len(T), 'languages,', len(ids), 'messages')
