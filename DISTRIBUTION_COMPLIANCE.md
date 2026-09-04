# Release compliance checklist

This is an engineering checklist, not legal advice. Have qualified counsel
review the final release where commercial distribution, employment ownership,
trademarks, or a jurisdiction-specific issue is involved.

Before publishing an ISO:

1. Record the exact source revision, build command, Buildroot configuration,
   package versions, and hashes of the ISO and source archive.
2. Generate Buildroot legal information for each architecture:
   `make -C toolchain O="$PWD/output/<arch>" legal-info`
   Publish the resulting `output/<arch>/legal-info/` directory with the ISO.
3. Publish complete corresponding source for every GPL-covered binary in the
   ISO, including `toolchain/`, `configs/`, `patches/`, scripts, and the exact
   COSMIC source revisions/submodules used by `scripts/build-cosmic.sh`.
4. Keep the MPL-2.0 source and notices for `support/pam-client` and the exact
   libcosmic revision available. Do not remove or alter their notices.
5. Include `LICENSE`, `THIRD_PARTY_NOTICES.md`, the rEFInd icon notice, and
   all generated legal information in the release archive and installed image.
6. Resolve `ASSET_PROVENANCE.md`. Remove any asset that lacks a documented
   source, author, and redistribution license.
7. Replace the generic MAN copyright holder with the real legal owner(s), and
   confirm that every contributor assigned or retained the rights claimed.
8. Do not use third-party names, logos, or trademarks in marketing without
   separate permission, even if the code license permits redistribution.

The ISO is not ready to be described as "all rights reserved" as a whole:
it includes open-source components with their own licenses. Only the scoped
MAN-authored material can carry that statement.
