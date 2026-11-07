#!/usr/bin/env python3
# Settings → Cloud Drives: adds the new strings to src/man-settings-po/LANG.po from
# tx-cloud-*.txt ("lang: s0|s1|...", in the order of KEYS). Buttons get their mnemonic
# underscore here, at the start of the translation.
import re, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
PO = HERE.parent.parent / 'src/man-settings-po'
sys.path.insert(0, str(PO))
import tool

KEYS = ["WebDAV (Nextcloud, ownCloud and others)", "Amazon S3 and Compatible Storage", "Windows Share (SMB)",
        "Cloud Drives", "Remove “%s”?",
        "Your files stay in the cloud. This computer stops showing them, and its copies of them are deleted.",
        "_Remove", "Connected", "Cannot connect. Trying again…", "Connecting…", "Not shown in Files", "Open in Files",
        "Server Address", "Server", "User Name", "Password", "Two-Factor Code", "Provider", "Server Type", "Region",
        "Endpoint", "Access Key ID", "Secret Access Key", "Account", "Key", "API Key", "Email", "Apple ID", "Domain",
        "Port", "Access", "Other", "_Back", "Could Not Add the Drive",
        "Check what you entered and your internet connection, then try again.", "Log In to %s",
        "_Open Login Page Again",
        "Your web browser opened the login page. Log in there and allow access, then come back here. This page continues by itself.",
        "C_ontinue", "One More Step", "Setting up the drive…", "Opening the login page…",
        "You log in with your web browser. Your password is never entered here.",
        "Enter the details of your account. They are kept in your home folder.", "Name in Files", "More Options",
        "_Log In", "C_onnect", "Cloud Drives Are Not Available",
        "rclone, which connects cloud drives, is not installed.", "_OK", "Search providers", "Popular",
        "All Providers", "Log in with your browser", "Add Cloud Drive",
        "Your cloud storage, in Files. Each drive connects by itself every time you log in.", "No cloud drives yet",
        "_Add Cloud Drive…", "Show Google Drive, OneDrive, Dropbox and other cloud storage in Files"]

msgs = tool.msgs()
missing = [k for k in KEYS if k not in msgs]
if missing:
    sys.exit(f'not in code: {missing}')
done = 0
for f in sorted(HERE.glob('tx-cloud-*.txt')):
    for ln in f.read_text(encoding='utf-8').splitlines():
        if not ln.strip():
            continue
        lang, _, rest = ln.partition(': ')
        parts = rest.split('|')
        if len(parts) != len(KEYS) or not all(p.strip() for p in parts):
            sys.exit(f'{lang}: {len(parts)} strings')
        tx = tool.read_po(PO / f'{lang}.po')
        for k, t in zip(KEYS, parts):
            if re.findall(r'%[a-z]', k) != re.findall(r'%[a-z]', t):
                sys.exit(f'{lang}: format in {t}')
            tx[k] = '_' + t if '_' in k else t
        lost = [m for m in msgs if m not in tx]
        if lost:
            sys.exit(f'{lang}: untranslated {lost[:3]}')
        tool.write_po(PO / f'{lang}.po', msgs, tx, lang)
        done += 1
print(done, 'languages,', len(msgs), 'messages')
