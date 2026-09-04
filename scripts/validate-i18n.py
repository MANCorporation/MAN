#!/usr/bin/env python3
"""Verify every chooser locale has every installer and OOBE translation key."""

import json
import re
import sys
import unicodedata
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "support/man-utilities/src/i18n.rs"
LOCALE_DIR = ROOT / "overlay/usr/share/man-utilities/i18n"
PROTECTED = ("MAN", "OOBE", "COSMIC", "FAT32", "ext2", "GPT", "MBR", "EFI", "QWERTY", "QWERTZ")

text = SOURCE.read_text()
language_block = text.split("pub static LANGUAGES:", 1)[1].split("];", 1)[0]
locales = re.findall(r'locale:\s*"([^"]+)"', language_block)
english_block = text.split("const ENGLISH:", 1)[1].split("];", 1)[0]
keys = list(dict.fromkeys(re.findall(r'\(\s*"([^"]+)",', english_block)))
english = {
    key: json.loads(f'"{raw}"')
    for key, raw in re.findall(r'\(\s*"([^"]+)",\s*"((?:[^"\\]|\\.)*)"\s*,?\s*\)', english_block)
}

errors = []
for locale in locales:
    path = LOCALE_DIR / f"{locale}.json"
    if not path.is_file():
        errors.append(f"{locale}: translation file is missing")
        continue
    try:
        values = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        errors.append(f"{locale}: invalid JSON: {error}")
        continue
    missing = [key for key in keys if key not in values or not values[key].strip()]
    extra = [key for key in values if key not in english]
    if missing:
        errors.append(f"{locale}: missing {len(missing)} keys: {', '.join(missing[:8])}")
    if extra:
        errors.append(f"{locale}: unknown keys: {', '.join(extra[:8])}")
    for key in keys:
        if key in values:
            expected = set(re.findall(r"\{[^{}]+\}", english[key]))
            actual = set(re.findall(r"\{[^{}]+\}", values[key]))
            if actual != expected:
                errors.append(f"{locale}:{key}: placeholder mismatch")
            for token in PROTECTED:
                if token in english[key] and token not in values[key]:
                    errors.append(f"{locale}:{key}: protected term {token!r} is missing")
            words = re.findall(r"\w+", values[key].lower())
            most_common = max(Counter(words).values(), default=0)
            visible = [
                character for character in values[key]
                if not character.isspace()
                and unicodedata.category(character) not in {"Mn", "Cf"}
            ]
            if len(values[key]) > max(220, len(english[key]) * 5 + 60):
                errors.append(f"{locale}:{key}: implausibly long translation")
            if len(words) > 12 and most_common * 2 > len(words):
                errors.append(f"{locale}:{key}: repeated-word translation degeneration")
            if len(values[key]) > 12 and len(visible) < 2:
                errors.append(f"{locale}:{key}: translation contains no visible text")

if errors:
    print("Translation validation failed:", file=sys.stderr)
    print("\n".join(f"  - {error}" for error in errors), file=sys.stderr)
    sys.exit(1)
print(f"Translation validation passed: {len(locales)} locales, {len(keys)} keys each")
