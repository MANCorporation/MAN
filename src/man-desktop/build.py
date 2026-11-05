#!/usr/bin/env python3
"""build.py SOURCES OUT.deb: MAN's Dock, Blur and Window Tiling from their upstream sources.

SOURCES holds the checkouts at the commits in UPSTREAM (see README.md). Each one stays
under its GPL: the COPYING text and every file's leading copyright comment are kept,
and README.md + the package's copyright file name the projects and these changes.
Everything else becomes part of MAN: uuids, settings, D-Bus names, what the user can read.
"""
import json, os, re, shutil, struct, subprocess, sys, tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SRC, OUT = Path(sys.argv[1]), Path(sys.argv[2])
VERSION = '1.0'
HOST = Path(os.environ.get('HOSTBIN', HERE.parent.parent / 'tools/hostbin/root'))

UPSTREAM = {
    'dock': ('dash-to-dock', 'https://github.com/ubuntu/dash-to-dock', '2dfae3474d', 'GPL-2'),
    'blur': ('blur-my-shell', 'https://github.com/aunetx/blur-my-shell', '309ebdca21', 'GPL-3'),
    'tiling': ('tilingshell', 'https://github.com/domferr/tilingshell', '18.0 (5c3c052cf0)', 'GPL-3'),
}

RENAMES = {
    'dock': [
        ('dash-to-dock@micxgx.gmail.com', 'man-dock@man.invalid'),
        ('org.gnome.shell.extensions.dash-to-dock', 'com.man.desktop.dock'),
        ('/org/gnome/shell/extensions/dash-to-dock/', '/com/man/desktop/dock/'),
        ('Dash to Dock', 'Dock'), ('Dash to dock', 'Dock'), ('Dash-to-Dock', 'Dock'), ('DashToDock', 'ManDock'), ('dashToDock', 'manDock'),
        ('dashtodock', 'mandock'), ('dash-to-dock', 'man-dock'),
    ],
    'blur': [
        ('blur-my-shell@aunetx', 'man-blur@man.invalid'),
        ('org.gnome.shell.extensions.blur-my-shell', 'com.man.desktop.blur'),
        ('/org/gnome/shell/extensions/blur-my-shell/', '/com/man/desktop/blur/'),
        ('dev.aunetx.BlurMyShell', 'com.man.Desktop.Blur'), ('/dev/aunetx/BlurMyShell', '/com/man/Desktop/Blur'),
        ('Blur my Shell', 'MAN Blur'), ('Blur My Shell', 'MAN Blur'), ('BlurMyShell', 'ManBlur'), ('blur-my-shell', 'man-blur'), ('blur_my_shell', 'man_blur'), ('bms-', 'man-blur-'), ('_bms_', '_man_blur_'),
        ('Dash to Dock', 'Dock'), ('DASH TO DOCK', 'DOCK'), ('dash to dock', 'dock'), ('dash_to_dock', 'man_dock'), ('DASH_TO_DOCK', 'MAN_DOCK'), ('DashToDock', 'ManDock'),
        ('dashtodock', 'mandock'), ('dash-to-dock', 'man-dock'),
    ],
    'tiling': [
        ('tilingshell@ferrarodomenico.com', 'man-tiling@man.invalid'),
        ('org.gnome.shell.extensions.tilingshell', 'com.man.desktop.tiling'),
        ('/org/gnome/shell/extensions/tilingshell/', '/com/man/desktop/tiling/'),
        ('/org/gnome/Shell/Extensions/tilingshell', '/com/man/desktop/tiling'),
        ('Tiling Shell', 'Window Tiling'), ('tilingshell', 'man-tiling'), ('TilingShell', 'ManTiling'), ('tilingShell', 'manTiling'),
    ],
}
TEXT = {'.js', '.css', '.scss', '.xml', '.json'}
PREFS_JS = "\n    // MAN: the settings live in the Settings app.\n    openPreferences() {\n        Gio.Subprocess.new(['gnome-control-center', '%s'], Gio.SubprocessFlags.NONE);\n    }\n"


def rename(text, rules):
    """Apply rules, leaving a file's leading comment (its copyright notice) as it is."""
    m = re.match(r'\s*(/\*.*?\*/\s*|(//[^\n]*\n\s*)+)', text, re.S)
    head = m.group(0) if m and re.search(r'copyright|license|gnu', m.group(0), re.I) else ''
    body = text[len(head):]
    for a, b in rules:
        body = body.replace(a, b)
    return head + body


def rename_tree(root, rules):
    for f in root.rglob('*'):
        if f.is_file() and f.suffix in TEXT:
            f.write_text(rename(f.read_text(encoding='utf-8'), rules), encoding='utf-8')


def sub(path, old, new):
    s = path.read_text(encoding='utf-8')
    assert old in s, f'{path}: {old!r} not found'
    path.write_text(s.replace(old, new), encoding='utf-8')


def add_prefs(extension_js, panel):
    """Give the extension class an openPreferences that opens Settings."""
    s = extension_js.read_text(encoding='utf-8')
    m = re.search(r'class \w+ extends (?:\w+\.)?Extension \{\n', s)
    assert m, extension_js
    s = s[:m.end()] + PREFS_JS % panel + s[m.end():]
    if not re.search(r"\bGio\b[^\n]*from", s):
        s = "import Gio from 'gi://Gio';\n" + s
    extension_js.write_text(s, encoding='utf-8')


def po_to_mo(po, mo):
    """A small msgfmt: plain msgid/msgstr pairs only (the Shell side uses no others here)."""
    pairs, cur, key = {}, {'msgid': '', 'msgstr': ''}, None
    def flush():
        if cur['msgid'] and cur['msgstr']:
            pairs[cur['msgid']] = cur['msgstr']
    unq = lambda l: bytes(l[1:-1], 'utf-8').decode('unicode_escape').encode('latin-1').decode('utf-8')
    for line in po.read_text(encoding='utf-8').splitlines():
        line = line.strip()
        if line.startswith('msgid '):
            flush(); cur = {'msgid': '', 'msgstr': ''}; key = 'msgid'; line = line[6:]
        elif line.startswith('msgstr '):
            key = 'msgstr'; line = line[7:]
        elif line.startswith(('msgid_plural', 'msgstr[', 'msgctxt')) :
            key = None; continue
        if line.startswith('"') and key:
            cur[key] += unq(line)
    flush()
    items = sorted((k.encode(), v.encode()) for k, v in pairs.items())
    keys, vals = b'', b''
    n = len(items); off = []; 
    for k, v in items:
        off.append((len(k), len(keys), len(v), len(vals)))
        keys += k + b'\0'; vals += v + b'\0'
    start = 28 + n * 16
    table = b''.join(struct.pack('<II', a, start + b) for a, b, _, _ in off)
    table += b''.join(struct.pack('<II', c, start + len(keys) + d) for _, _, c, d in off)
    mo.parent.mkdir(parents=True, exist_ok=True)
    mo.write_bytes(struct.pack('<7I', 0x950412de, 0, n, 28, 28 + n * 8, 0, 0) + table + keys + vals)


def metadata(path, uuid, name, description, schema, domain):
    d = {'uuid': uuid, 'name': name, 'description': description, 'shell-version': ['51'], 'settings-schema': schema, 'gettext-domain': domain}
    old = json.loads(path.read_text()) if path.exists() else {}
    if 'session-modes' in old:
        d['session-modes'] = old['session-modes']
    path.write_text(json.dumps(d, indent=2) + '\n')


def build_dock(stage, schemas, locale):
    src = SRC / 'dash-to-dock'
    d = stage / 'man-dock@man.invalid'
    d.mkdir(parents=True)
    for f in src.iterdir():
        if f.suffix == '.js' and f.name != 'prefs.js' or f.name in ('COPYING', 'dependencies', '_stylesheet.scss', 'metadata.json'):
            (shutil.copytree if f.is_dir() else shutil.copy)(f, d / f.name)
    rename_tree(d, RENAMES['dock'])
    env = dict(os.environ, LD_LIBRARY_PATH=str(HOST / 'usr/lib/x86_64-linux-gnu'))
    subprocess.run([HOST / 'usr/bin/sassc', d / '_stylesheet.scss', d / 'stylesheet.css'], check=True, env=env)
    (d / '_stylesheet.scss').unlink()
    # The menu of the app grid button: no section name, and it opens Settings.
    sub(d / 'appIcons.js', "this.addMenuItem(new PopupMenu.PopupSeparatorMenuItem(__('Dock')));\n", '')
    add_prefs(d / 'extension.js', 'multitasking')
    metadata(d / 'metadata.json', 'man-dock@man.invalid', 'MAN Dock', "MAN's dock.", 'com.man.desktop.dock', 'man-dock')
    x = schemas / 'com.man.desktop.dock.gschema.xml'
    x.write_text(rename((src / 'schemas/org.gnome.shell.extensions.dash-to-dock.gschema.xml').read_text(), RENAMES['dock']))
    for po in (src / 'po').glob('*.po'):
        po_to_mo(po, locale / po.stem / 'LC_MESSAGES/man-dock.mo')


def build_blur(stage, schemas):
    src = SRC / 'blur-my-shell'
    d = stage / 'man-blur@man.invalid'
    shutil.copytree(src / 'src', d)
    shutil.rmtree(d / 'preferences')
    (d / 'prefs.js').unlink()
    shutil.rmtree(d / 'styles')
    order = re.search(r'STYLESHEETS = (.*?)\n\n', (src / 'Makefile').read_text(), re.S).group(1)
    css = ''.join((src / s).read_text() for s in order.replace('\\\n', ' ').split())
    (d / 'stylesheet.css').write_text(css)
    shutil.copy(src / 'LICENSE', d / 'COPYING')
    shutil.copy(src / 'metadata.json', d / 'metadata.json')
    rename_tree(d, RENAMES['blur'])
    (d / 'components/dock/dash_to_dock.js').rename(d / 'components/dock/man_dock.js')
    metadata(d / 'metadata.json', 'man-blur@man.invalid', 'MAN Blur', "MAN's blur effects.", 'com.man.desktop.blur', 'man-blur')
    x = schemas / 'com.man.desktop.blur.gschema.xml'
    x.write_text(rename((src / 'schemas/org.gnome.shell.extensions.blur-my-shell.gschema.xml').read_text(), RENAMES['blur']))


def build_tiling(stage, schemas, locale):
    """From the release build of 18.0: its JavaScript is the plain output of the TypeScript."""
    src = SRC / 'tilingshell-release'
    d = stage / 'man-tiling@man.invalid'
    shutil.copytree(src, d)
    for n in ('prefs.js', 'prefs.css', 'resources.gresource', 'schemas'):
        p = d / n
        shutil.rmtree(p) if p.is_dir() else p.unlink()
    shutil.copy(SRC / 'tilingshell/LICENSE', d / 'COPYING')
    rename_tree(d, RENAMES['tiling'])
    (d / 'components/windowManager/tilingShellWindowManager.js').rename(d / 'components/windowManager/manTilingWindowManager.js')
    add_prefs(d / 'extension.js', 'multitasking')
    metadata(d / 'metadata.json', 'man-tiling@man.invalid', 'MAN Window Tiling', "MAN's window tiling.", 'com.man.desktop.tiling', 'man-tiling')
    for mo in (d / 'locale').rglob('*.mo'):
        target = locale / mo.parent.parent.name / 'LC_MESSAGES/man-tiling.mo'
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(mo, target)
    shutil.rmtree(d / 'locale')
    # Its icons: same files, under MAN's resource path.
    work = Path(tempfile.mkdtemp())
    shutil.copytree(SRC / 'tilingshell/gresources', work / 'g')
    sub(work / 'g/resources.gresource.xml', '/org/gnome/Shell/Extensions/tilingshell', '/com/man/desktop/tiling')
    subprocess.run([os.environ.get('GLIB_COMPILE_RESOURCES', 'glib-compile-resources'), '--sourcedir', work / 'g', '--target', d / 'resources.gresource', work / 'g/resources.gresource.xml'], check=True,
                   env=dict(os.environ, PATH=str(HOST / 'usr/bin') + ':' + os.environ['PATH'], LD_LIBRARY_PATH=str(HOST / 'usr/lib/x86_64-linux-gnu')))
    x = schemas / 'com.man.desktop.tiling.gschema.xml'
    x.write_text(rename((src / 'schemas/org.gnome.shell.extensions.tilingshell.gschema.xml').read_text(), RENAMES['tiling']))


def add_notice(folder, key):
    name, url, rev, lic = UPSTREAM[key]
    (folder / 'NOTICE').write_text(f"This program is a modified version of {name} ({url}, {rev}), licensed under the {lic} (see COPYING).\nMAN changed its names, settings, menus and defaults and removed its preferences window (November 2026).\nThe changes are in src/man-desktop of MAN's repository; /usr/share/doc/man-desktop-shell/copyright has how to get the source.\n")


def check_no_names(root):
    """Nothing but license texts and leading copyright comments may name the projects."""
    bad = re.compile(r'dash.to.dock|dashtodock|blur.my.shell|tiling.?shell|aunetx|micxgx|domferr|ferrarodomenico|ko-fi|patreon', re.I)
    found = []
    for f in root.rglob('*'):
        if not f.is_file() or f.name in ('COPYING', 'NOTICE', 'copyright') or f.suffix not in TEXT | {'.desktop'}:
            continue
        s = f.read_text(encoding='utf-8', errors='replace')
        m = re.match(r'\s*(/\*.*?\*/\s*|(//[^\n]*\n\s*)+)', s, re.S)
        if m and re.search(r'copyright|license|gnu', m.group(0), re.I):
            s = s[len(m.group(0)):]
        for line in s.splitlines():
            if bad.search(line):
                found.append(f'{f.relative_to(root)}: {line.strip()[:110]}')
    return found


root = Path(tempfile.mkdtemp())
ext = root / 'usr/share/gnome-shell/extensions'
schemas = root / 'usr/share/glib-2.0/schemas'
locale = root / 'usr/share/locale'
schemas.mkdir(parents=True)
build_dock(ext, schemas, locale)
build_blur(ext, schemas)
build_tiling(ext, schemas, locale)
shutil.copytree(HERE / 'man-look', ext / 'man-look@man.invalid')
for key in UPSTREAM:
    add_notice(ext / f'man-{key}@man.invalid', key)
shutil.copy(HERE / '20_man-desktop-shell.gschema.override', schemas)
problems = check_no_names(root)
if problems:
    print('\n'.join(problems[:60]))
    print(len(problems), 'traces left')
doc = root / 'usr/share/doc/man-desktop-shell'
doc.mkdir(parents=True)
text = "MAN's Dock, Blur and Window Tiling are modified versions of these GNU GPL programs;\nthe license texts are next to the programs (COPYING in each folder under\n/usr/share/gnome-shell/extensions). MAN changed names, settings and menus, removed their\npreference windows, and sets its own defaults. MAN Corporation provides the source on\nrequest for three years; the changes are in src/man-desktop of MAN's repository.\n\n"
text += ''.join(f'  {u[0]}: {u[1]}, {u[2]}, {u[3]}\n' for u in UPSTREAM.values())
(doc / 'copyright').write_text(text)
(root / 'DEBIAN').mkdir()
(root / 'DEBIAN/control').write_text(f'''Package: man-desktop-shell
Version: {VERSION}
Architecture: all
Maintainer: MAN Developers <dev@man.local>
Section: gnome
Priority: optional
Depends: gnome-shell (>= 51~), gnome-shell (<< 52~), gir1.2-dbusmenu-glib-0.4
Description: MAN's dock, blur effects, window tiling and system dialog look
 Parts of MAN's desktop. They are set up in Settings.
''')
(root / 'DEBIAN/postinst').write_text('#!/bin/sh\nset -e\nif [ "$1" = configure ]; then\n    glib-compile-schemas /usr/share/glib-2.0/schemas\nfi\n')
(root / 'DEBIAN/postinst').chmod(0o755)
for dirpath, dirs, files in os.walk(root):
    os.chmod(dirpath, 0o755)
    for f in files:
        p = Path(dirpath) / f
        if p.name != 'postinst':
            p.chmod(0o644)
OUT.parent.mkdir(parents=True, exist_ok=True)
subprocess.run(['dpkg-deb', '--root-owner-group', '-Zxz', '-b', root, OUT], check=True, stdout=subprocess.DEVNULL)
print('built', OUT, sum(1 for _ in root.rglob('*') if _.is_file()), 'files')
sys.exit(1 if problems else 0)
