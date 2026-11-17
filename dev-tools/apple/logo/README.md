# MAN logo for the Mac boot loader (m1n1)

`man_128.png` and `man_256.png`: the MAN logo in the sizes m1n1 shows at start-up
(128 px on normal screens, 256 px on Retina screens). They are made for m1n1's
`LOGO=<name>` build option, the same way Debian's m1n1 package ships
`debian_128.png` and `debian_256.png`.

MAN does not build m1n1 with AI tools: m1n1's maintainers forbid the use of AI or LLM
tools with their project (https://asahilinux.org/slop/). Building m1n1 with this logo is
left to a person, following m1n1's own build instructions. Two builds show a logo:
the first stage (inside the Asahi installer, `boot/m1n1.bin`) and the second stage
(`/usr/lib/m1n1/m1n1.bin` in the MAN image, which `update-m1n1` puts on the EFI
partition).
