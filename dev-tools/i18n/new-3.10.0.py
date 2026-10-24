# man-installer 3.10.0: App Installer + TEST builds. The new strings' indices
# shift the old ones, so tx/LANG.txt is rebuilt: old translations by message
# (msgs-3.9.0.json), new ones from tx310_*.py (lists in KEYS order).
#   python3 new-3.10.0.py [--check]      then: python3 import.py LANG tx/LANG.txt
import glob, importlib, json, os, re, sys
sys.path.insert(0, '/home/kristihack/Documents/MANRUST/man-installer/po')
from extract import messages

HERE = os.path.dirname(os.path.abspath(__file__))
KEYS = [
"This file is not an app that can be installed here (.deb, .flatpak, .flatpakref or AppImage).",
"This app is made for {arch} computers and does not work on this one (x86-64).",
"The file could not be read. It may be damaged.",
"This is not an AppImage.",
"Introduction",
"License",
"Destination Select",
"Welcome to the {name} Installer",
"Software License Agreement",
"Do You Agree to the License?",
"To continue installing, you must agree to the terms of the software license agreement.",
"_Read License",
"_Agree",
"Select a Destination",
"Standard Install",
"Debian package (.deb)",
"Flatpak app",
"Flatpak app from {source}",
"AppImage",
"You will be guided through the steps necessary to install this software.",
"Publisher",
"Type",
"Size",
"Website",
"File",
"This app is installed already. Installing it again replaces it.",
"Apps from .deb packages become part of the system. When MAN is updated they are removed and need to be installed again. Flatpak apps and AppImages are kept.",
"Only install apps from sources you trust.",
"Where do you want to install {name}?",
"Only your account can use the app.",
"Needs administrator rights.",
"Only for Me",
"For All Users of This Computer",
"This will take {size} of space on your computer.",
"The app is downloaded from {source}.",
"The app is installed for all users of this computer.",
"The app is installed only for you.",
"The AppImage is moved to {folder} and added to your apps.",
"Click Install to install {name}.",
"Checking what else the app needs…",
"Also installs the packages it needs: {list}",
"Some packages the app needs are not available yet. Installing gets the newest package lists first; if they are still missing, the app cannot be installed.",
"Installing {name}…",
"Preparing…",
"Installing the package…",
"Moving the app to its folder…",
"The app could not be installed.",
"Installing the app with Flatpak…",
"_Close",
"The Installation Was Successful",
"The app was installed.",
"The Installation Failed",
"_Open",
"App Installer",
"Go _Back",
"Reading the app…",
"Install {name}",
"Apps",
"Choose an App to Install",
"Getting the newest package lists…",
"Copying the app",
"Adding the app to the menu",
"The file was not found.",
"The package could not be installed.",
"The app's menu entry is not valid.",
"Package Installer",
"Install apps from .deb, Flatpak and AppImage files",
]


def new_translations():
    out = {}
    for f in sorted(glob.glob(os.path.join(HERE, 'tx310_*.py'))):
        mod = importlib.import_module(os.path.basename(f)[:-3])
        out.update(mod.L)
    return out


def check(T):
    ph = lambda s: sorted(re.findall(r'\{[a-z_]+\}', s))
    bad = []
    for lang, tr in T.items():
        if len(tr) != len(KEYS):
            bad.append(f'{lang}: {len(tr)} strings, want {len(KEYS)}'); continue
        for k, t in zip(KEYS, tr):
            if ph(k) != ph(t):
                bad.append(f'{lang}: placeholders in {t!r}')
            if ('_' in k) != ('_' in t):
                bad.append(f'{lang}: mnemonic in {t!r}')
            if not t.strip():
                bad.append(f'{lang}: empty for {k!r}')
    return bad


if __name__ == '__main__':
    sys.path.insert(0, HERE)
    T = new_translations()
    bad = check(T)
    if bad:
        print('\n'.join(bad)); sys.exit(1)
    if '--check' in sys.argv:
        print(len(T), 'languages ok'); sys.exit(0)
    old_ids = json.load(open(os.path.join(HERE, 'msgs-3.9.0.json')))
    ids = list(messages())
    assert all(k in ids for k in KEYS), [k for k in KEYS if k not in ids]
    json.dump(ids, open(os.path.join(HERE, 'msgs-3.10.0.json'), 'w'), ensure_ascii=False, indent=0)
    for f in sorted(glob.glob(os.path.join(HERE, 'tx', '*.txt'))):
        lang = os.path.basename(f)[:-4]
        old = {}
        for ln in open(f, encoding='utf-8'):
            n, _, t = ln.rstrip('\n').partition('|')
            if n.isdigit():
                old[old_ids[int(n)]] = t
        new = dict(zip(KEYS, (t.replace('\n', '\\n') for t in T[lang])))
        lines = [f'{i}|{new.get(m, old.get(m))}' for i, m in enumerate(ids)]
        missing = [m for m in ids if m not in new and m not in old]
        if missing:
            sys.exit(f'{lang}: missing {missing}')
        open(f, 'w', encoding='utf-8').write('\n'.join(lines) + '\n')
    print(len(T), 'languages,', len(ids), 'messages')
