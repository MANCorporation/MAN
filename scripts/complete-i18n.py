#!/usr/bin/env python3
"""Complete MAN locale JSON files with the NLLB-200 offline translator.

Existing non-English translations are retained. Missing keys and values that
are still identical to the English source are translated in batches.
"""

import json
import re
from pathlib import Path

import torch
from transformers import AutoModelForSeq2SeqLM, AutoTokenizer

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "support/man-utilities/src/i18n.rs"
LOCALE_DIR = ROOT / "overlay/usr/share/man-utilities/i18n"
MODEL = "facebook/nllb-200-distilled-600M"
PROTECTED = ("MAN", "OOBE", "COSMIC", "FAT32", "ext2", "GPT", "MBR", "EFI", "QWERTY", "QWERTZ")
PROTECTED_PATTERN = re.compile("(" + "|".join(map(re.escape, PROTECTED)) + ")")

NLLB = {
    "af": "afr_Latn", "ar": "arb_Arab", "az": "azj_Latn",
    "bg": "bul_Cyrl", "bn": "ben_Beng", "cs": "ces_Latn",
    "da": "dan_Latn", "de": "deu_Latn", "el": "ell_Grek",
    "en": "eng_Latn", "eo": "epo_Latn", "es": "spa_Latn",
    "fa": "pes_Arab", "fi": "fin_Latn", "fr": "fra_Latn",
    "he": "heb_Hebr", "hi": "hin_Deva", "hr": "hrv_Latn",
    "hu": "hun_Latn", "id": "ind_Latn", "it": "ita_Latn",
    "ja": "jpn_Jpan", "kk": "kaz_Cyrl", "ko": "kor_Hang",
    "la": "lat_Latn", "mn": "khk_Cyrl", "mr": "mar_Deva",
    "ms": "zsm_Latn", "nl": "nld_Latn", "no": "nob_Latn",
    "pl": "pol_Latn", "pt": "por_Latn", "ro": "ron_Latn",
    "ru": "rus_Cyrl", "sk": "slk_Latn", "sr": "srp_Cyrl",
    "sv": "swe_Latn", "sw": "swh_Latn", "ta": "tam_Taml",
    "te": "tel_Telu", "th": "tha_Thai", "tl": "tgl_Latn",
    "tr": "tur_Latn", "uk": "ukr_Cyrl", "ur": "urd_Arab",
    "uz": "uzn_Latn", "vi": "vie_Latn", "zh_CN": "zho_Hans",
    "zh_TW": "zho_Hant",
}


def english_strings():
    text = SOURCE.read_text()
    block = text.split("const ENGLISH:", 1)[1].split("];", 1)[0]
    result = {}
    for key, raw in re.findall(r'\(\s*"([^"]+)",\s*"((?:[^"\\]|\\.)*)"\s*,?\s*\)', block):
        result.setdefault(key, json.loads(f'"{raw}"'))
    return result


def supported_locales():
    text = SOURCE.read_text()
    block = text.split("pub static LANGUAGES:", 1)[1].split("];", 1)[0]
    return re.findall(r'locale:\s*"([^"]+)"', block)


def target_for(locale):
    if locale in NLLB:
        return NLLB[locale]
    return NLLB[locale.split("_", 1)[0]]


def placeholders(value):
    return set(re.findall(r"\{[^{}]+\}", value))


def protect(value):
    for index, token in enumerate(PROTECTED):
        value = value.replace(token, f"ZXQTERM{index}QXZ")
    return value


def restore(value):
    for index, token in enumerate(PROTECTED):
        value = value.replace(f"ZXQTERM{index}QXZ", token)
    return value


def lost_protected_term(source, translated):
    return any(token in source and token not in translated for token in PROTECTED)


def main():
    canonical = english_strings()
    tokenizer = AutoTokenizer.from_pretrained(MODEL, src_lang="eng_Latn")
    model = AutoModelForSeq2SeqLM.from_pretrained(MODEL)
    model.eval()

    for locale in supported_locales():
        path = LOCALE_DIR / f"{locale}.json"
        current = json.loads(path.read_text())
        if locale.startswith("en_"):
            completed = dict(canonical)
        else:
            needed = [
                key for key, value in canonical.items()
                if key not in current or current[key] == value
                or lost_protected_term(value, current[key])
            ]
            translated = {}
            target = target_for(locale)
            for start in range(0, len(needed), 16):
                keys = needed[start:start + 16]
                texts = [protect(canonical[key]) for key in keys]
                encoded = tokenizer(texts, return_tensors="pt", padding=True,
                                    truncation=True, max_length=192)
                with torch.inference_mode():
                    output = model.generate(
                        **encoded,
                        forced_bos_token_id=tokenizer.convert_tokens_to_ids(target),
                        max_new_tokens=192,
                        num_beams=1,
                    )
                values = [restore(value) for value in tokenizer.batch_decode(output, skip_special_tokens=True)]
                for key, value in zip(keys, values):
                    if lost_protected_term(canonical[key], value):
                        chunks = PROTECTED_PATTERN.split(canonical[key])
                        plain = [chunk for chunk in chunks if chunk and chunk not in PROTECTED]
                        translated_plain = iter(())
                        if plain:
                            enc = tokenizer(plain, return_tensors="pt", padding=True,
                                            truncation=True, max_length=192)
                            with torch.inference_mode():
                                out = model.generate(
                                    **enc,
                                    forced_bos_token_id=tokenizer.convert_tokens_to_ids(target),
                                    max_new_tokens=192,
                                    num_beams=1,
                                )
                            translated_plain = iter(tokenizer.batch_decode(out, skip_special_tokens=True))
                        value = "".join(
                            chunk if chunk in PROTECTED else next(translated_plain)
                            for chunk in chunks if chunk
                        )
                    # Translation must not silently remove formatting tokens.
                    if placeholders(value) != placeholders(canonical[key]):
                        chunks = re.split(r"(\{[^{}]+\})", canonical[key])
                        plain = [chunk for chunk in chunks if chunk and not chunk.startswith("{")]
                        enc = tokenizer(plain, return_tensors="pt", padding=True)
                        with torch.inference_mode():
                            out = model.generate(
                                **enc,
                                forced_bos_token_id=tokenizer.convert_tokens_to_ids(target),
                                max_new_tokens=192,
                                num_beams=1,
                            )
                        translated_plain = iter(tokenizer.batch_decode(out, skip_special_tokens=True))
                        value = "".join(
                            chunk if chunk.startswith("{") else next(translated_plain)
                            for chunk in chunks if chunk
                        )
                    # Some very low-resource language pairs can return an
                    # empty sequence. Never write an unusable UI label; keep
                    # the source visible until that entry is curated.
                    translated[key] = value.strip() or canonical[key]
            completed = {
                key: translated.get(key, current.get(key, source))
                for key, source in canonical.items()
            }
        path.write_text(json.dumps(completed, ensure_ascii=False, indent=2) + "\n")
        print(f"{locale}: {len(completed)} strings")


if __name__ == "__main__":
    main()
