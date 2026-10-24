DUHACH! - install MAN from Windows, no USB stick needed
=======================================================

1. Download the MAN ISO.
2. Double-click DUHACH!.exe and allow it to make changes (it needs
   administrator rights to change partitions and the boot menu).
3. Choose the MAN ISO, choose how much space MAN gets, and press
   "Prepare MAN". DUHACH! shrinks drive C:, copies the MAN installer to a
   small partition and restarts your PC into it.
4. In the MAN installer choose "Install Alongside Windows".

After that, a menu at every start lets you choose MAN or Windows.

Requirements: 64-bit Windows 10 or 11 started in UEFI mode, a GPT disk,
Secure Boot turned off, and at least 40 GB free on C:.
Back up your important files first.

Run DUHACH! again to remove the MAN installer and give the space back to
Windows if you change your mind before installing.

DUHACH! is (c) 2026 MAN Corporation, all rights reserved; see LICENSE.txt.
It uses GTK, libadwaita and other open-source libraries from the MSYS2
project and Rust libraries, under their own licenses: see
THIRD-PARTY-NOTICES.txt, THIRD-PARTY-RUST.txt and the licenses folder.
