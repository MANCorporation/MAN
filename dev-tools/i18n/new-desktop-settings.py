#!/usr/bin/env python3
# Adds the Dock/Blur/Window Tiling strings of Settings to src/man-settings-po/LANG.po
# from tx-desktop-settings.txt ("lang: s0|s1|...", in KEYS order).
import json, subprocess, sys, tempfile
from pathlib import Path
HERE = Path(__file__).resolve().parent
PO = HERE.parent.parent / 'src/man-settings-po'
KEYS = ["Show Dock", "Hide Automatically", "The dock makes room when a window needs the space", "Bottom", "Dock", "Position", "Icon Size", "Show on All Displays", "Show Trash", "Show Drives", "Blur Effects", "Turn this off on slow computers", "Blur", "Top Bar", "Activities Overview", "Menus", "Lock Screen", "Window Tiling", "Arrange windows in layouts. Choose and edit layouts with the button in the top bar.", "Snap Assistant", "Drag a window to the top of the screen to choose its place", "Snap at Screen Edges", "Automatic Tiling", "New windows fill the layout by themselves", "Show Tiling Button", "Space Between Windows", "Space at Screen Edges"]
tx = {}
for ln in (HERE / 'tx-desktop-settings.txt').read_text(encoding='utf-8').splitlines():
    lang, _, rest = ln.partition(': ')
    parts = rest.split('|')
    if len(parts) != len(KEYS) or not all(p.strip() for p in parts):
        sys.exit(f'{lang}: {len(parts)} strings')
    tx[lang] = dict(zip(KEYS, parts))
g = {'__name__': 'tool', '__file__': str(PO / 'tool.py')}
src = (PO / 'tool.py').read_text(encoding='utf-8')
exec(compile(src.split("if __name__ ==")[0], 'tool.py', 'exec'), g)
msgs = g['msgs']()
missing = [k for k in KEYS if k not in msgs]
if missing:
    sys.exit(f'not in code: {missing}')
langs = sorted(p.stem for p in PO.glob('*.po'))
if set(langs) != set(tx):
    sys.exit(f'languages differ: {set(langs) ^ set(tx)}')
for lang in langs:
    old = g['read_po'](PO / f'{lang}.po')
    old.update(tx[lang])
    lost = [m for m in msgs if m not in old]
    if lost:
        sys.exit(f'{lang}: untranslated {lost[:3]}')
    g['write_po'](PO / f'{lang}.po', msgs, old, lang)
print(len(langs), 'languages,', len(msgs), 'messages')
