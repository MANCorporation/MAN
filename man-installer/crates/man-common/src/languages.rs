//! Languages offered by the installer and MAN Setup: the system locale, the
//! name in the language itself, its word for "Welcome" and its main country.

pub struct Language {
    /// glibc locale, e.g. "cs_CZ.UTF-8".
    pub locale: &'static str,
    /// Name in the language itself.
    pub native: &'static str,
    /// Name in English (shown under the native name, used for searching).
    pub english: &'static str,
    pub welcome: &'static str,
    /// ISO 3166 code of the country preselected after choosing it.
    pub country: &'static str,
}

const fn l(locale: &'static str, native: &'static str, english: &'static str, welcome: &'static str, country: &'static str) -> Language {
    Language { locale, native, english, welcome, country }
}

pub const DEFAULT_LOCALE: &str = "en_US.UTF-8";

pub const LANGUAGES: &[Language] = &[
    l("en_US.UTF-8", "English (United States)", "English (United States)", "Welcome", "US"),
    l("en_GB.UTF-8", "English (United Kingdom)", "English (United Kingdom)", "Welcome", "GB"),
    l("af_ZA.UTF-8", "Afrikaans", "Afrikaans", "Welkom", "ZA"),
    l("sq_AL.UTF-8", "Shqip", "Albanian", "Mirë se vini", "AL"),
    l("am_ET.UTF-8", "አማርኛ", "Amharic", "እንኳን ደህና መጡ", "ET"),
    l("ar_EG.UTF-8", "العربية", "Arabic", "أهلاً وسهلاً", "EG"),
    l("hy_AM.UTF-8", "Հայերեն", "Armenian", "Բարի գալուստ", "AM"),
    l("az_AZ.UTF-8", "Azərbaycanca", "Azerbaijani", "Xoş gəlmisiniz", "AZ"),
    l("eu_ES.UTF-8", "Euskara", "Basque", "Ongi etorri", "ES"),
    l("be_BY.UTF-8", "Беларуская", "Belarusian", "Вітаем", "BY"),
    l("bn_BD.UTF-8", "বাংলা", "Bengali", "স্বাগতম", "BD"),
    l("bs_BA.UTF-8", "Bosanski", "Bosnian", "Dobrodošli", "BA"),
    l("bg_BG.UTF-8", "Български", "Bulgarian", "Добре дошли", "BG"),
    l("ca_ES.UTF-8", "Català", "Catalan", "Benvinguts", "ES"),
    l("zh_CN.UTF-8", "简体中文", "Chinese (Simplified)", "欢迎", "CN"),
    l("zh_TW.UTF-8", "繁體中文", "Chinese (Traditional)", "歡迎", "TW"),
    l("hr_HR.UTF-8", "Hrvatski", "Croatian", "Dobrodošli", "HR"),
    l("cs_CZ.UTF-8", "Čeština", "Czech", "Vítejte", "CZ"),
    l("da_DK.UTF-8", "Dansk", "Danish", "Velkommen", "DK"),
    l("nl_NL.UTF-8", "Nederlands", "Dutch", "Welkom", "NL"),
    l("eo.UTF-8", "Esperanto", "Esperanto", "Bonvenon", ""),
    l("et_EE.UTF-8", "Eesti", "Estonian", "Tere tulemast", "EE"),
    l("fil_PH.UTF-8", "Filipino", "Filipino", "Maligayang pagdating", "PH"),
    l("fi_FI.UTF-8", "Suomi", "Finnish", "Tervetuloa", "FI"),
    l("fr_FR.UTF-8", "Français", "French", "Bienvenue", "FR"),
    l("gl_ES.UTF-8", "Galego", "Galician", "Benvido", "ES"),
    l("ka_GE.UTF-8", "ქართული", "Georgian", "მოგესალმებით", "GE"),
    l("de_DE.UTF-8", "Deutsch", "German", "Willkommen", "DE"),
    l("el_GR.UTF-8", "Ελληνικά", "Greek", "Καλώς ήρθατε", "GR"),
    l("gu_IN.UTF-8", "ગુજરાતી", "Gujarati", "સ્વાગત છે", "IN"),
    l("he_IL.UTF-8", "עברית", "Hebrew", "ברוכים הבאים", "IL"),
    l("hi_IN.UTF-8", "हिन्दी", "Hindi", "स्वागत है", "IN"),
    l("hu_HU.UTF-8", "Magyar", "Hungarian", "Üdvözöljük", "HU"),
    l("is_IS.UTF-8", "Íslenska", "Icelandic", "Velkomin", "IS"),
    l("id_ID.UTF-8", "Bahasa Indonesia", "Indonesian", "Selamat datang", "ID"),
    l("ga_IE.UTF-8", "Gaeilge", "Irish", "Fáilte", "IE"),
    l("it_IT.UTF-8", "Italiano", "Italian", "Benvenuto", "IT"),
    l("ja_JP.UTF-8", "日本語", "Japanese", "ようこそ", "JP"),
    l("kn_IN.UTF-8", "ಕನ್ನಡ", "Kannada", "ಸ್ವಾಗತ", "IN"),
    l("kk_KZ.UTF-8", "Қазақ тілі", "Kazakh", "Қош келдіңіз", "KZ"),
    l("ko_KR.UTF-8", "한국어", "Korean", "환영합니다", "KR"),
    l("lv_LV.UTF-8", "Latviešu", "Latvian", "Laipni lūdzam", "LV"),
    l("lt_LT.UTF-8", "Lietuvių", "Lithuanian", "Sveiki atvykę", "LT"),
    l("mk_MK.UTF-8", "Македонски", "Macedonian", "Добредојдовте", "MK"),
    l("ms_MY.UTF-8", "Bahasa Melayu", "Malay", "Selamat datang", "MY"),
    l("ml_IN.UTF-8", "മലയാളം", "Malayalam", "സ്വാഗതം", "IN"),
    l("mr_IN.UTF-8", "मराठी", "Marathi", "स्वागत आहे", "IN"),
    l("mn_MN.UTF-8", "Монгол", "Mongolian", "Тавтай морилно уу", "MN"),
    l("ne_NP.UTF-8", "नेपाली", "Nepali", "स्वागत छ", "NP"),
    l("nb_NO.UTF-8", "Norsk bokmål", "Norwegian Bokmål", "Velkommen", "NO"),
    l("fa_IR.UTF-8", "فارسی", "Persian", "خوش آمدید", "IR"),
    l("pl_PL.UTF-8", "Polski", "Polish", "Witamy", "PL"),
    l("pt_BR.UTF-8", "Português (Brasil)", "Portuguese (Brazil)", "Bem-vindo", "BR"),
    l("pt_PT.UTF-8", "Português (Portugal)", "Portuguese (Portugal)", "Bem-vindo", "PT"),
    l("pa_IN.UTF-8", "ਪੰਜਾਬੀ", "Punjabi", "ਜੀ ਆਇਆਂ ਨੂੰ", "IN"),
    l("ro_RO.UTF-8", "Română", "Romanian", "Bun venit", "RO"),
    l("ru_RU.UTF-8", "Русский", "Russian", "Добро пожаловать", "RU"),
    l("sr_RS.UTF-8", "Српски", "Serbian", "Добродошли", "RS"),
    l("si_LK.UTF-8", "සිංහල", "Sinhala", "ආයුබෝවන්", "LK"),
    l("sk_SK.UTF-8", "Slovenčina", "Slovak", "Vitajte", "SK"),
    l("sl_SI.UTF-8", "Slovenščina", "Slovenian", "Dobrodošli", "SI"),
    l("es_ES.UTF-8", "Español (España)", "Spanish (Spain)", "Bienvenido", "ES"),
    l("es_MX.UTF-8", "Español (Latinoamérica)", "Spanish (Latin America)", "Bienvenido", "MX"),
    l("sw_KE.UTF-8", "Kiswahili", "Swahili", "Karibu", "KE"),
    l("sv_SE.UTF-8", "Svenska", "Swedish", "Välkommen", "SE"),
    l("ta_IN.UTF-8", "தமிழ்", "Tamil", "வரவேற்கிறோம்", "IN"),
    l("te_IN.UTF-8", "తెలుగు", "Telugu", "స్వాగతం", "IN"),
    l("th_TH.UTF-8", "ไทย", "Thai", "ยินดีต้อนรับ", "TH"),
    l("tr_TR.UTF-8", "Türkçe", "Turkish", "Hoş geldiniz", "TR"),
    l("uk_UA.UTF-8", "Українська", "Ukrainian", "Ласкаво просимо", "UA"),
    l("ur_PK.UTF-8", "اردو", "Urdu", "خوش آمدید", "PK"),
    l("uz_UZ.UTF-8", "Oʻzbekcha", "Uzbek", "Xush kelibsiz", "UZ"),
    l("vi_VN.UTF-8", "Tiếng Việt", "Vietnamese", "Chào mừng", "VN"),
    l("cy_GB.UTF-8", "Cymraeg", "Welsh", "Croeso", "GB"),
];

pub fn language(locale: &str) -> Option<&'static Language> {
    LANGUAGES.iter().find(|l| l.locale == locale)
}

/// A locale name the system accepts: "ll_CC.UTF-8" or "ll.UTF-8", with an
/// optional "@modifier".
pub fn valid_locale(s: &str) -> bool {
    let (base, modifier) = s.split_once('@').unwrap_or((s, ""));
    let Some(lang) = base.strip_suffix(".UTF-8") else { return false };
    let mut parts = lang.split('_');
    let ll = parts.next().unwrap_or("");
    let cc = parts.next();
    parts.next().is_none()
        && (2..=3).contains(&ll.len())
        && ll.chars().all(|c| c.is_ascii_lowercase())
        && cc.is_none_or(|c| c.len() == 2 && c.chars().all(|c| c.is_ascii_uppercase()))
        && modifier.chars().all(|c| c.is_ascii_alphanumeric())
}

/// ISO 3166 alpha-2 code.
pub fn valid_country(s: &str) -> bool {
    s.len() == 2 && s.chars().all(|c| c.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_consistent() {
        assert!(LANGUAGES.len() >= 60);
        for l in LANGUAGES {
            assert!(valid_locale(l.locale), "{}", l.locale);
            assert!(l.country.is_empty() || valid_country(l.country), "{}", l.country);
            assert_eq!(LANGUAGES.iter().filter(|o| o.locale == l.locale).count(), 1, "{}", l.locale);
        }
        assert!(language(DEFAULT_LOCALE).is_some());
        assert!(!valid_locale("cs_CZ.UTF-8; rm -rf /"));
        assert!(valid_locale("sr_RS.UTF-8@latin"));
    }
}
