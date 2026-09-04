# Third-party notices

This file records material included in the MAN source tree or produced by its
release build. It is not a substitute for the complete package-level legal
information generated for a particular ISO; see `DISTRIBUTION_COMPLIANCE.md`.

| Component | Location/use | License status and required action |
| --- | --- | --- |
| Buildroot | `toolchain/`, build system | GPL-2.0-or-later. Keep its `COPYING`; the installed notice bundle includes it as `GPL-2.0-or-later.txt`. Publish the matching Buildroot tree and MAN build configuration with a release. |
| COSMIC desktop projects | downloaded and built by `scripts/build-cosmic.sh`; patched by `patches/` | Several shipped COSMIC programs, including cosmic-comp, cosmic-session, cosmic-greeter, cosmic-store, and cosmic-files, are GPL-3.0-only. The installed notice bundle includes `GPL-3.0.txt`. Provide the exact corresponding source, including every MAN patch and build script, for the binary release. |
| libcosmic | dependency of MAN Utilities and MAN Guide | MPL-2.0. Preserve its notice and make the relevant source available as required by MPL-2.0. The locked revision is in `support/man-utilities/Cargo.lock`. |
| `pam-client` | `support/pam-client/` | MPL-2.0 by Christoph Grenz and contributors. Its upstream license is retained verbatim as `MPL-2.0-pam-client.txt`. |
| rEFInd 0.14.1 | `scripts/bootloader/*.EFI`, EFI configuration and ext2 drivers | GPL-3.0-or-later. MAN's string-branding changes do not make it proprietary. The installed notice bundle includes `GPL-3.0.txt`; publish the matching rEFInd source and the exact branding source with releases. |
| AwOken and rEFInd icon assets | `scripts/bootloader/icons/` | `arrow_left.png` and `arrow_right.png` are modified AwOken 2.5 assets, licensed CC-BY-SA 3.0 by Alessandro Roncone. `transparent.png` is rEFInd artwork and is distributed here under its CC-BY-SA 3.0 option. Preserve the author, source URL, license URI, and modification notice in `rEFInd-ICON-NOTICES.md`. |
| Haiku icon artwork | `ICONS/`, `overlay/usr/share/icons/`, `theme_/` | PNG exports of Haiku artwork, MIT-licensed except for trademarked Haiku marks. Preserve the Haiku MIT notice (`Haiku-ICONS-MIT.txt`) and do not use the Haiku name, logo, or leaf marks as MAN branding. |
| Rust crates and Buildroot packages | lockfiles and build output | Mixed licenses. Generate and publish the architecture-specific legal-information bundle before release. |

The installed operating system stores this notice bundle at
`/usr/share/doc/MAN/`. The SPDX identifiers above describe the component
license; the actual license texts shipped there are authoritative.
