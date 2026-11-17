#!/usr/bin/env python3
# man-installer 3.17.0: MAN Recovery (and the Mac texts of 3.16) in man-installer/po/LANG.po,
# from tx-recovery-*.txt ("lang: s0|s1|...", in KEYS order). Mnemonic versions ("_Restart")
# are made from the plain text. "MAN", "Flatpak" and the macOS menu label stay as they are.
import re, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
PO = HERE.parent.parent / 'man-installer/po'
sys.path.insert(0, str(PO))
from extract import messages, write_po
KEYS = ["Erase an External Disk and Install MAN", "Everything on the chosen external disk is deleted. The Mac's internal disk is not shown and not changed.", "Install on This Mac", "Recommended. Into the {size} prepared in macOS. macOS and its partitions are not changed.", "No space for MAN was prepared yet. In macOS, run MAN-Mac/install-man.sh from the MAN USB stick and choose “MAN boot loader + space for MAN”.", "This Mac (space prepared in macOS)", "MAN Recovery", "Restore your system, reinstall MAN or open a Terminal. First, an administrator of this computer signs in.", "Administrator", "No installed system was found on this disk. You can reinstall MAN.", "This computer has MAN {version}.", "What Would You Like to Do?", "Restore a Snapshot", "Your system broke? Go back to a moment when everything worked. Your files stay.", "Download the newest MAN and install it again. Your files, accounts and settings stay.", "Terminal", "For experts: type commands with full control of the computer.", "There are no snapshots on this computer.", "Choose a moment when everything worked. The files in your home folder stay as they are now.", "Restore “{name}”?", "Your system goes back to how it was on {date}. Your files stay. The system as it is now is kept as a snapshot, so you can come back to it.", "Restoring Your System", "Your system was restored. Restart to use it.", "MAN Recovery downloads the newest MAN. Connect to a network first.", "Looking for the newest MAN…", "Your files, accounts and settings stay.", "Apps you installed as Flatpaks stay; other apps you added may need to be installed again.", "The system as it is now is kept as a snapshot.", "Keep the computer plugged in. This takes a while.", "_Reinstall MAN", "MAN {version} ({size}) will be downloaded and installed on this computer.", "No MAN for this computer was found.", "Reinstalling MAN", "MAN was installed again. Your files and accounts were kept. Restart to use it.", "Do not turn the computer off.", "That Did Not Work", "_Details", "_Restart", "_Back to MAN Recovery", "_Shut Down", "Looking at this computer…", "Preparing MAN Recovery", "Keeping your accounts and settings", "Downloading MAN", "Opening the downloaded MAN", "Leaving out what Recovery does not need", "Setting up the recovery system", "Adding its start-up files", "MAN Recovery is ready", "This MAN has no MAN Recovery program.", "MAN Recovery was not started.", "The installed system has no kernel for MAN Recovery.", "MAN Recovery was not started from the boot menu.", "The disk of this MAN was not found.", "This works only in MAN Recovery.", "The password is not correct.", "No MAN for this computer was found on MAN's download page.", "The downloaded MAN is damaged (its checksum does not match). Try again.", "The partition of this MAN was not found.", "Restore or reinstall MAN"]
MNEMONIC = {'_Reinstall MAN', '_Details', '_Restart', '_Back to MAN Recovery', '_Shut Down'}
src = (PO / 'build-mo.py').read_text(encoding='utf-8').split('if __name__')[0]
g = {'__file__': str(PO / 'build-mo.py'), '__name__': 'tool'}
exec(compile(src, 'build-mo.py', 'exec'), g)
msgs = messages()
for k in KEYS:
    if k not in msgs:
        sys.exit(f'not in code: {k}')
done = 0
for f in sorted(HERE.glob('tx-recovery-*.txt')):
    for ln in f.read_text(encoding='utf-8').splitlines():
        if not ln.strip():
            continue
        lang, _, rest = ln.partition(': ')
        parts = [p.strip() for p in rest.split('|')]
        if len(parts) != len(KEYS) or not all(parts):
            sys.exit(f'{lang}: {len(parts)} strings')
        tx = dict(g['read_po'](PO / f'{lang}.po'))
        for k, t in zip(KEYS, parts):
            if sorted(re.findall(r'\{[a-z_]+\}', k)) != sorted(re.findall(r'\{[a-z_]+\}', t)):
                sys.exit(f'{lang}: placeholders in {t}')
            if k in MNEMONIC:
                # Japanese, Korean, Chinese: the English access key after the text, as GNOME does.
                key = k[k.index('_') + 1]
                t = f'{t}(_{key})' if lang in ('ja', 'ko', 'zh_CN', 'zh_TW') else '_' + t
            tx[k] = t
        lost = [m for m in msgs if m not in tx]
        if lost:
            sys.exit(f'{lang}: untranslated {lost[:3]}')
        write_po(str(PO / f'{lang}.po'), msgs, tx, lang)
        done += 1
print(done, 'languages,', len(msgs), 'messages')
