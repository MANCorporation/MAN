-----BEGIN PGP SIGNED MESSAGE-----
Hash: SHA512

Format: 3.0 (quilt)
Source: grub2
Binary: grub2, grub-linuxbios, grub-efi, grub-common, grub2-common, grub-emu, grub-emu-dbg, grub-pc-bin, grub-pc-dbg, grub-pc, grub-rescue-pc, grub-coreboot-bin, grub-coreboot-dbg, grub-coreboot, grub-efi-ia32-bin, grub-efi-ia32-unsigned, grub-efi-ia32-dbg, grub-efi-ia32, grub-efi-ia32-signed-template, grub-efi-amd64-bin, grub-efi-amd64-unsigned, grub-efi-amd64-dbg, grub-efi-amd64, grub-efi-amd64-signed-template, grub-efi-ia64-bin, grub-efi-ia64-unsigned, grub-efi-ia64-dbg, grub-efi-ia64, grub-efi-arm-bin, grub-efi-arm-unsigned, grub-efi-arm-dbg, grub-efi-arm, grub-efi-arm64-bin, grub-efi-arm64-unsigned, grub-efi-arm64-dbg, grub-efi-arm64, grub-efi-arm64-signed-template, grub-efi-riscv64-bin, grub-efi-riscv64-unsigned, grub-efi-riscv64-dbg, grub-efi-riscv64, grub-efi-loong64-bin, grub-efi-loong64-unsigned, grub-efi-loong64-dbg, grub-efi-loong64, grub-ieee1275-bin, grub-ieee1275-dbg, grub-ieee1275, grub-firmware-qemu, grub-uboot-bin, grub-uboot-dbg, grub-uboot, grub-xen-bin,
 grub-xen-dbg, grub-xen, grub-xen-host, grub-yeeloong-bin, grub-yeeloong-dbg, grub-yeeloong, grub-theme-starfield,
 grub-mount-udeb
Architecture: any
Version: 2.12-9+deb13u2
Maintainer: GRUB Maintainers <pkg-grub-devel@alioth-lists.debian.net>
Uploaders: Felix Zielcke <fzielcke@z-51.de>, Jordi Mallach <jordi@debian.org>, Steve McIntyre <93sam@debian.org>, Julian Andres Klode <jak@debian.org>, Mate Kukri <mate.kukri@canonical.com>
Homepage: https://www.gnu.org/software/grub/
Standards-Version: 3.9.6
Vcs-Browser: https://salsa.debian.org/grub-team/grub
Vcs-Git: https://salsa.debian.org/grub-team/grub.git
Build-Depends: debhelper-compat (= 13), patchutils, python3, flex, bison, gawk, po-debconf, help2man, texinfo, gcc-multilib [i386 kopensolaris-i386 any-amd64 any-ppc64 any-sparc], xfonts-unifont, libfreetype6-dev, gettext, libdevmapper-dev [linux-any], libgeom-dev (>= 8.2+ds1-1~) [kfreebsd-any] | libgeom-dev (<< 8.2) [kfreebsd-any], libsdl2-dev [!hurd-any], xorriso, qemu-system [i386 kfreebsd-i386 kopensolaris-i386 any-amd64], cpio [i386 kopensolaris-i386 amd64 x32], parted [!hurd-any], libfuse3-dev [linux-any kfreebsd-any], fonts-dejavu-core, liblzma-dev, liblzo2-dev, lzop, dosfstools [any-i386 any-amd64 any-arm64], squashfs-tools [any-i386 any-arm any-amd64 any-arm64 any-ia64 any-loong64 any-riscv64], wamerican, libparted-dev [any-powerpc any-ppc64 any-ppc64el], pkgconf, bash-completion, libefiboot-dev [i386 amd64 ia64 x32 armel armhf arm64 riscv64 loong64], libefivar-dev [i386 amd64 ia64 x32 armel armhf arm64 riscv64 loong64]
Build-Conflicts: autoconf2.13, libnvpair-dev, libzfs-dev
Package-List:
 grub-common deb admin optional arch=any
 grub-coreboot deb admin optional arch=any-i386,any-amd64
 grub-coreboot-bin deb admin optional arch=any-i386,any-amd64
 grub-coreboot-dbg deb debug optional arch=any-i386,any-amd64
 grub-efi deb admin optional arch=any-i386,any-amd64,any-arm64,any-ia64,any-arm,any-riscv64,any-loong64
 grub-efi-amd64 deb admin optional arch=i386,kopensolaris-i386,any-amd64
 grub-efi-amd64-bin deb admin optional arch=i386,kopensolaris-i386,any-amd64
 grub-efi-amd64-dbg deb debug optional arch=i386,kopensolaris-i386,any-amd64
 grub-efi-amd64-signed-template deb admin optional arch=amd64
 grub-efi-amd64-unsigned deb admin optional arch=i386,kopensolaris-i386,any-amd64
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
 grub-efi-ia64 deb admin optional arch=any-ia64
 grub-efi-ia64-bin deb admin optional arch=any-ia64
 grub-efi-ia64-dbg deb debug optional arch=any-ia64
 grub-efi-ia64-unsigned deb admin optional arch=any-ia64
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
 grub-mount-udeb udeb debian-installer optional arch=linux-any,kfreebsd-any
 grub-pc deb admin optional arch=any-i386,any-amd64
 grub-pc-bin deb admin optional arch=any-i386,any-amd64
 grub-pc-dbg deb debug optional arch=any-i386,any-amd64
 grub-rescue-pc deb admin optional arch=any-i386,any-amd64
 grub-theme-starfield deb admin optional arch=any-i386,any-amd64,any-powerpc,any-ppc64,any-ppc64el,any-sparc,any-sparc64,any-mipsel,any-ia64,any-arm,any-arm64,any-riscv64,any-loong64
 grub-uboot deb admin optional arch=any-arm
 grub-uboot-bin deb admin optional arch=any-arm
 grub-uboot-dbg deb debug optional arch=any-arm
 grub-xen deb admin optional arch=i386,amd64
 grub-xen-bin deb admin optional arch=i386,amd64
 grub-xen-dbg deb debug optional arch=i386,amd64
 grub-xen-host deb admin optional arch=i386,amd64
 grub-yeeloong deb admin optional arch=any-mipsel
 grub-yeeloong-bin deb admin optional arch=any-mipsel
 grub-yeeloong-dbg deb debug optional arch=any-mipsel
 grub2 deb oldlibs optional arch=any-i386,any-amd64,any-powerpc,any-ppc64,any-ppc64el,any-sparc,any-sparc64
 grub2-common deb admin optional arch=any-i386,any-amd64,any-powerpc,any-ppc64,any-ppc64el,any-sparc,any-sparc64,any-mipsel,any-ia64,any-arm,any-arm64,any-riscv64,any-loong64
Checksums-Sha1:
 9a5cd9860a02d479ff65461b710a4d85ea46b9f4 6675608 grub2_2.12.orig.tar.xz
 ad610a474548de8cfb22128cad0ecb6cb5c42d85 1123180 grub2_2.12-9+deb13u2.debian.tar.xz
Checksums-Sha256:
 f3c97391f7c4eaa677a78e090c7e97e6dc47b16f655f04683ebd37bef7fe0faa 6675608 grub2_2.12.orig.tar.xz
 4d3c4689ee100466a36b6a5caa2477ef525fbc5b6ec27cb0319c83db7d9faf89 1123180 grub2_2.12-9+deb13u2.debian.tar.xz
Files:
 60c564b1bdc39d8e43b3aab4bc0fb140 6675608 grub2_2.12.orig.tar.xz
 cce3fff8f4007e1dc115b413b5289c5c 1123180 grub2_2.12-9+deb13u2.debian.tar.xz

-----BEGIN PGP SIGNATURE-----

iQIzBAEBCgAdFiEEUryGlb40+QrX1Ay4E4jA+JnoM2sFAmn3ebkACgkQE4jA+Jno
M2tnHhAAlsIMbAt9K3v351+73v39oheggV4ERb8arsoGl3WoR0Ub1c/bF5ShzHOK
FTkLxKLKMpB89wuuEYFKV3fnyiuCuFMPVBxw427YQuD5zOx43l0vYeNGgne0HzSj
+lM4J9/3WUZHqSPZ1hAfvgXsKASIAS/Xqx9pSwAAWeZsiK6HCpdWpvEULVmBJDIT
nPdyqlzpaNtcFsmJ+SU/C7Dntg2G1KTFfDjf8k02n7Sn9O0FatnoOhUK10WkbqXD
0IPQ76NhIL9zWDCUlb52O119vcYDERRR0sZFtFEo89wiefDfKDFCOce3KGk/BXVO
kUw6jkTDQ9lZeauXUgJeFlDfT3l3nuwx8wbXHSNGoThEag9oR9Z3sTQ2yWAKtap3
MsyaKCS6Drchbf6M+3I3Y6J5FamI06vXO6Qm/J7YZT4HrSZPurGrpGHIkIw3UaEh
0kD4zY5YvrXYHVZryOy5CY9IeolOO7ydJtrBt+XaBD/6LwX6lo0OH45CGly5OZ58
7cwAjDJUy9FlpgpOF+bSXZLLtnDly3zWbGfr4fIzRZi8IQie7yrJi1TQxyUchMRY
rAi0fWn7IXyQENv7x5f3QQwp5afD6Ro2l6mVPPcNoUQ2+DoSPlj2nMABmOTPHSHi
QPsZzYPlXt6RP8Xe5l38o7fai13gxA5PsielKkSxxmOaFWJ4QHw=
=bFl1
-----END PGP SIGNATURE-----
