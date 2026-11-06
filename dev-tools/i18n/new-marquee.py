#!/usr/bin/env python3
# man-installer 3.14.0: adds the Marquee strings to man-installer/po/LANG.po from
# tx-marquee-*.txt ("lang: s0|s1|...", in KEYS order). Mnemonic versions ("_Save") are
# made from the plain text. "Marquee" stays as it is in every language.
import re, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
PO = HERE.parent.parent / 'man-installer/po'
sys.path.insert(0, str(PO))
from extract import messages, write_po
KEYS = ["Movies", "Music", "Photos", "Sharing as “{name}” at {address}", "Sharing as “{name}”. This computer is not connected to a network yet.", "Other computers cannot see your libraries.", "Sharing could not start.", "The libraries could not be saved.", "Stop Sharing", "Stop Sharing “{name}”?", "The files stay on this computer. Other computers just cannot see this library any more.", "“{name}” is shared on this network.", "Edit Library", "New Library", "Name", "Folders and Files", "Add Folder…", "Add Files…", "_Save", "C_reate", "Nothing added yet", "Remove", "Shared Libraries", "Create Library", "Share Your Media", "Choose folders or files, and other computers on this network can play them in Marquee.", "Anyone on this network can play what you share here, without a password, so share only on networks you trust. On the other computer, open Marquee and go to Network.", "Share on This Network", "Create _Shared Library", "_Open Marquee", "Welcome to Marquee", "Would you like to open Marquee, or share your movies, music and photos with other computers on this network?", "Play and share movies, music and photos"]
MNEMONIC = {'_Save', 'C_reate', 'Create _Shared Library', '_Open Marquee'}
src = (PO / 'build-mo.py').read_text(encoding='utf-8').split('if __name__')[0]
g = {'__file__': str(PO / 'build-mo.py'), '__name__': 'tool'}
exec(compile(src, 'build-mo.py', 'exec'), g)
msgs = messages()
for k in KEYS:
    if k not in msgs:
        sys.exit(f'not in code: {k}')
done = 0
for f in sorted(HERE.glob('tx-marquee-*.txt')):
    for ln in f.read_text(encoding='utf-8').splitlines():
        lang, _, rest = ln.partition(': ')
        parts = rest.split('|')
        if len(parts) != len(KEYS) or not all(p.strip() for p in parts):
            sys.exit(f'{lang}: {len(parts)} strings')
        tx = dict(g['read_po'](PO / f'{lang}.po'))
        for k, t in zip(KEYS, parts):
            if sorted(re.findall(r'\{[a-z_]+\}', k)) != sorted(re.findall(r'\{[a-z_]+\}', t)):
                sys.exit(f'{lang}: placeholders in {t}')
            tx[k] = '_' + t if k in MNEMONIC else t
        tx['_Stop Sharing'] = '_' + tx['Stop Sharing']
        tx['Shared _Libraries'] = '_' + tx['Shared Libraries']
        tx['Marquee'] = 'Marquee'
        lost = [m for m in msgs if m not in tx]
        if lost:
            sys.exit(f'{lang}: untranslated {lost[:3]}')
        write_po(str(PO / f'{lang}.po'), msgs, tx, lang)
        done += 1
print(done, 'languages,', len(msgs), 'messages')
