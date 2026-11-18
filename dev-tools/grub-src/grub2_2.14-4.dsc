-----BEGIN PGP SIGNED MESSAGE-----
Hash: SHA512

Format: 3.0 (quilt)
Source: grub2
Binary: grub2, grub-linuxbios, grub-efi, grub-common, grub2-common, grub-emu, grub-emu-dbg, grub-pc-bin, grub-pc-dbg, grub-pc, grub-rescue-pc, grub-coreboot-bin, grub-coreboot-dbg, grub-coreboot, grub-efi-ia32-bin, grub-efi-ia32-unsigned, grub-efi-ia32-dbg, grub-efi-ia32, grub-efi-ia32-signed-template, grub-efi-amd64-bin, grub-efi-amd64-unsigned, grub-efi-amd64-dbg, grub-efi-amd64, grub-efi-amd64-signed-template, grub-efi-arm-bin, grub-efi-arm-unsigned, grub-efi-arm-dbg, grub-efi-arm, grub-efi-arm64-bin, grub-efi-arm64-unsigned, grub-efi-arm64-dbg, grub-efi-arm64, grub-efi-arm64-signed-template, grub-efi-riscv64-bin, grub-efi-riscv64-unsigned, grub-efi-riscv64-dbg, grub-efi-riscv64, grub-efi-loong64-bin, grub-efi-loong64-unsigned, grub-efi-loong64-dbg, grub-efi-loong64, grub-ieee1275-bin, grub-ieee1275-dbg, grub-ieee1275, grub-firmware-qemu, grub-uboot-bin, grub-uboot-dbg, grub-uboot, grub-xen-bin, grub-xen-dbg, grub-xen, grub-xen-host, grub-theme-starfield, grub-mount-udeb
Architecture: any
Version: 2.14-4
Maintainer: GRUB Maintainers <pkg-grub-devel@alioth-lists.debian.net>
Uploaders: Felix Zielcke <fzielcke@z-51.de>, Jordi Mallach <jordi@debian.org>, Steve McIntyre <93sam@debian.org>, Julian Andres Klode <jak@debian.org>, Mate Kukri <mate.kukri@canonical.com>
Homepage: https://www.gnu.org/software/grub/
Standards-Version: 3.9.6
Vcs-Browser: https://salsa.debian.org/grub-team/grub
Vcs-Git: https://salsa.debian.org/grub-team/grub.git
Build-Depends: debhelper-compat (= 13), patchutils, python3, flex, bison, gawk, po-debconf, help2man, texinfo, gcc-multilib [i386 any-amd64 any-ppc64 any-sparc], xfonts-unifont, libfreetype6-dev, gettext, libdevmapper-dev [linux-any], libsdl2-dev [!hurd-any], xorriso, qemu-system [i386 any-amd64] <!nocheck>, cpio [i386 amd64 x32], parted [!hurd-any], libfuse3-dev [linux-any], fonts-dejavu-core, liblzma-dev, liblzo2-dev, lzop, dosfstools [any-i386 any-amd64 any-arm64], squashfs-tools [any-i386 any-arm any-amd64 any-arm64 any-loong64 any-riscv64], wamerican, libparted-dev [any-powerpc any-ppc64 any-ppc64el], pkgconf, bash-completion, libefiboot-dev [i386 amd64 x32 armel armhf arm64 riscv64 loong64], libefivar-dev [i386 amd64 x32 armel armhf arm64 riscv64 loong64], autoconf-archive
Build-Conflicts: autoconf2.13, libnvpair-dev, libzfs-dev
Package-List:
 grub-common deb admin optional arch=any
 grub-coreboot deb admin optional arch=any-i386,any-amd64
 grub-coreboot-bin deb admin optional arch=any-i386,any-amd64
 grub-coreboot-dbg deb debug optional arch=any-i386,any-amd64
 grub-efi deb admin optional arch=any-i386,any-amd64,any-arm64,any-arm,any-riscv64,any-loong64
 grub-efi-amd64 deb admin optional arch=i386,any-amd64
 grub-efi-amd64-bin deb admin optional arch=i386,any-amd64
 grub-efi-amd64-dbg deb debug optional arch=i386,any-amd64
 grub-efi-amd64-signed-template deb admin optional arch=amd64
 grub-efi-amd64-unsigned deb admin optional arch=i386,any-amd64
 grub-efi-arm deb admin optional arch=any-arm
 grub-efi-arm-bin deb admin optional arch=any-arm
 grub-efi-arm-dbg deb debug optional arch=any-arm
 grub-efi-arm-unsigned deb admin optional arch=any-arm
 grub-efi-arm64 deb admin optional arch=any-arm64
 grub-efi-arm64-bin deb admin optional arch=any-arm64
 grub-efi-arm64-dbg deb debug optional arch=any-arm64
 grub-efi-arm64-signed-template deb admin optional arch=arm64
 grub-efi-arm64-unsigned deb admin optional arch=any-arm64
 grub-efi-ia32 deb admin optional arch=any-i386,any-amd64
 grub-efi-ia32-bin deb admin optional arch=any-i386,any-amd64
 grub-efi-ia32-dbg deb debug optional arch=any-i386,any-amd64
 grub-efi-ia32-signed-template deb admin optional arch=i386
 grub-efi-ia32-unsigned deb admin optional arch=any-i386,any-amd64
 grub-efi-loong64 deb admin optional arch=any-loong64
 grub-efi-loong64-bin deb admin optional arch=any-loong64
 grub-efi-loong64-dbg deb debug optional arch=any-loong64
 grub-efi-loong64-unsigned deb admin optional arch=any-loong64
 grub-efi-riscv64 deb admin optional arch=any-riscv64
 grub-efi-riscv64-bin deb admin optional arch=any-riscv64
 grub-efi-riscv64-dbg deb debug optional arch=any-riscv64
 grub-efi-riscv64-unsigned deb admin optional arch=any-riscv64
 grub-emu deb admin optional arch=any-i386,any-amd64,any-powerpc
 grub-emu-dbg deb debug optional arch=any-i386,any-amd64,any-powerpc
 grub-firmware-qemu deb admin optional arch=any-i386,any-amd64
 grub-ieee1275 deb admin optional arch=any-i386,any-amd64,any-powerpc,any-ppc64,any-ppc64el,any-sparc,any-sparc64
 grub-ieee1275-bin deb admin optional arch=any-i386,any-amd64,any-powerpc,any-ppc64,any-ppc64el,any-sparc,any-sparc64
 grub-ieee1275-dbg deb debug optional arch=any-i386,any-amd64,any-powerpc,any-ppc64,any-ppc64el,any-sparc,any-sparc64
 grub-linuxbios deb oldlibs optional arch=any-i386,any-amd64
 grub-mount-udeb udeb debian-installer optional arch=linux-any
 grub-pc deb admin optional arch=any-i386,any-amd64
 grub-pc-bin deb admin optional arch=any-i386,any-amd64
 grub-pc-dbg deb debug optional arch=any-i386,any-amd64
 grub-rescue-pc deb admin optional arch=any-i386,any-amd64
 grub-theme-starfield deb admin optional arch=any-i386,any-amd64,any-powerpc,any-ppc64,any-ppc64el,any-sparc,any-sparc64,any-mipsel,any-arm,any-arm64,any-riscv64,any-loong64
 grub-uboot deb admin optional arch=any-arm
 grub-uboot-bin deb admin optional arch=any-arm
 grub-uboot-dbg deb debug optional arch=any-arm
 grub-xen deb admin optional arch=i386,amd64
 grub-xen-bin deb admin optional arch=i386,amd64
 grub-xen-dbg deb debug optional arch=i386,amd64
 grub-xen-host deb admin optional arch=i386,amd64
 grub2 deb oldlibs optional arch=any-i386,any-amd64,any-powerpc,any-ppc64,any-ppc64el,any-sparc,any-sparc64
 grub2-common deb admin optional arch=any
Checksums-Sha1:
 b8370fa991f63027764f41d58b90aa8a0871f1e9 7725668 grub2_2.14.orig.tar.xz
 90ab4a47debd62b1b5c1f2c3aeb23021a30b8d98 1099216 grub2_2.14-4.debian.tar.xz
Checksums-Sha256:
 bc8d3c73535b8838d8c8e2654d73edc4e6ae8c8acdb45d5df5dc9a1547446d43 7725668 grub2_2.14.orig.tar.xz
 290779e500e4de6e68f45d89853cde3a7873a42d037b5980376bb85b1b42137d 1099216 grub2_2.14-4.debian.tar.xz
Files:
 383f9effad01c235d2535357ff717543 7725668 grub2_2.14.orig.tar.xz
 60dedd7284ef808eb6226f8d365b6558 1099216 grub2_2.14-4.debian.tar.xz

-----BEGIN PGP SIGNATURE-----

iQJFBAEBCgAvFiEEzrtSMB1hfpEDkP4WWHl5VzRCaE4FAmq0+ZIRHDkzc2FtQGRl
Ymlhbi5vcmcACgkQWHl5VzRCaE5vAg//bJdRzepdD/wmDsAC2iReFsx3od5ZwCnB
0baziGgiqL9yuBosdEpC2oG6ToO/SYPkE8tr2eFWbfNXmDbpcw1pc9RFLKIY3CEO
Q7bzTV1lAQX3w1oTpeq4fbEWxwcJfZbUgDToDK1qdX2h95WS2+pmJGtW4cK6qqkB
JFPEP4B1cT29o3qSNGz7YQ6B836kDQujz7b8np8ALCzgYF5A5mn/QsC3O5ALw6IJ
8gFSnM+jL2k6WnOTRHxEt4Wv3eOl76OAcPl2Z7+y1m/YWbo4PZfDrpVB5wpa1RY7
hdwJJra9uSWnC3A+9OKWIvc8p84bh/tOPgb1O32WsOMRij8vFeelQMouOrTGrGzS
Q/Z/bkOUGVVMpcUDujKbTf0V2hYkJQxVPBv55XmGUQMCic2HJ32uoyIIbo460BYr
g1DZHA8M+LgubTqRBnmToNIn5kK4PvaN8/eZe2syTYAPfqR+ibbVIA/snKFWYIIE
lD2cEGfa1JWQ3n9onah8Sv4WSqAQ413VK92Vccrs0Bfr9GSnaDiD5l0TyAmVqnaI
pHuTo0sRNyaoMbDnQk7UX79mT5RgxI4fF6NFlFQ+SX/hrliwdHY9yjxE8l6R9qnI
Oh44gTRAapd280S7qXBSsLhwx8VOUgPHUu6IlDh24lc0Y1wzBB5pc+w0aFqm1BiW
4aoWp0rfuhw=
=sbKw
-----END PGP SIGNATURE-----
