//! Internationalization module for MAN Utilities.
//!
//! Provides a simple JSON-based translation system. The selected language is
//! persisted to `/var/lib/man/man-language` so the next launch picks it up
//! automatically. Translation files live in
//! `/usr/share/man-utilities/i18n/<LANG>.json`.
//!
//! Each JSON file maps translation keys to localized strings:
//! ```json
//! { "app_title": "MAN Utilities", "install_man": "Install MAN" }
//! ```
//!
//! At runtime `tr("install_man")` returns the translated string, falling
//! back to the English default when the key or language file is missing.

use std::collections::HashMap;
use std::sync::OnceLock;

/// A supported language.
pub struct Language {
    pub locale: &'static str,
    pub name: &'static str,
    pub native: &'static str,
    /// Icon name for the country flag (installed in the hicolor icon theme).
    pub flag: &'static str,
}

/// Languages with a complete installer and OOBE translation catalog.
pub static LANGUAGES: &[Language] = &[
    Language {
        locale: "en_US",
        name: "English (United States)",
        native: "English",
        flag: "flag-us",
    },
    Language {
        locale: "en_GB",
        name: "English (United Kingdom)",
        native: "English",
        flag: "flag-gb",
    },
    Language {
        locale: "es_ES",
        name: "Spanish (Spain)",
        native: "Español",
        flag: "flag-es",
    },
    Language {
        locale: "es_MX",
        name: "Spanish (Mexico)",
        native: "Español",
        flag: "flag-mx",
    },
    Language {
        locale: "fr_FR",
        name: "French (France)",
        native: "Français",
        flag: "flag-fr",
    },
    Language {
        locale: "fr_CA",
        name: "French (Canada)",
        native: "Français",
        flag: "flag-ca",
    },
    Language {
        locale: "de_DE",
        name: "German (Germany)",
        native: "Deutsch",
        flag: "flag-de",
    },
    Language {
        locale: "it_IT",
        name: "Italian (Italy)",
        native: "Italiano",
        flag: "flag-it",
    },
    Language {
        locale: "pt_BR",
        name: "Portuguese (Brazil)",
        native: "Português do Brasil",
        flag: "flag-br",
    },
    Language {
        locale: "pt_PT",
        name: "Portuguese (Portugal)",
        native: "Português",
        flag: "flag-pt",
    },
    Language {
        locale: "ru_RU",
        name: "Russian (Russia)",
        native: "Русский",
        flag: "flag-ru",
    },
    Language {
        locale: "ja_JP",
        name: "Japanese (Japan)",
        native: "日本語",
        flag: "flag-jp",
    },
    Language {
        locale: "ko_KR",
        name: "Korean (South Korea)",
        native: "한국어",
        flag: "flag-kr",
    },
    Language {
        locale: "zh_CN",
        name: "Chinese (Simplified, China)",
        native: "简体中文",
        flag: "flag-cn",
    },
    Language {
        locale: "zh_TW",
        name: "Chinese (Traditional, Taiwan)",
        native: "繁體中文",
        flag: "flag-tw",
    },
    Language {
        locale: "ar_SA",
        name: "Arabic (Saudi Arabia)",
        native: "العربية",
        flag: "flag-sa",
    },
    Language {
        locale: "ar_EG",
        name: "Arabic (Egypt)",
        native: "العربية",
        flag: "flag-eg",
    },
    Language {
        locale: "tr_TR",
        name: "Turkish (Turkey)",
        native: "Türkçe",
        flag: "flag-tr",
    },
    Language {
        locale: "pl_PL",
        name: "Polish (Poland)",
        native: "Polski",
        flag: "flag-pl",
    },
    Language {
        locale: "nl_NL",
        name: "Dutch (Netherlands)",
        native: "Nederlands",
        flag: "flag-nl",
    },
    Language {
        locale: "sv_SE",
        name: "Swedish (Sweden)",
        native: "Svenska",
        flag: "flag-se",
    },
    Language {
        locale: "da_DK",
        name: "Danish (Denmark)",
        native: "Dansk",
        flag: "flag-dk",
    },
    Language {
        locale: "no_NO",
        name: "Norwegian (Norway)",
        native: "Norsk",
        flag: "flag-no",
    },
    Language {
        locale: "fi_FI",
        name: "Finnish (Finland)",
        native: "Suomi",
        flag: "flag-fi",
    },
    Language {
        locale: "cs_CZ",
        name: "Czech (Czech Republic)",
        native: "Čeština",
        flag: "flag-cz",
    },
    Language {
        locale: "sk_SK",
        name: "Slovak (Slovakia)",
        native: "Slovenčina",
        flag: "flag-sk",
    },
    Language {
        locale: "hu_HU",
        name: "Hungarian (Hungary)",
        native: "Magyar",
        flag: "flag-hu",
    },
    Language {
        locale: "ro_RO",
        name: "Romanian (Romania)",
        native: "Română",
        flag: "flag-ro",
    },
    Language {
        locale: "bg_BG",
        name: "Bulgarian (Bulgaria)",
        native: "Български",
        flag: "flag-bg",
    },
    Language {
        locale: "hr_HR",
        name: "Croatian (Croatia)",
        native: "Hrvatski",
        flag: "flag-hr",
    },
    Language {
        locale: "sr_RS",
        name: "Serbian (Serbia)",
        native: "Српски",
        flag: "flag-rs",
    },
    Language {
        locale: "uk_UA",
        name: "Ukrainian (Ukraine)",
        native: "Українська",
        flag: "flag-ua",
    },
    Language {
        locale: "el_GR",
        name: "Greek (Greece)",
        native: "Ελληνικά",
        flag: "flag-gr",
    },
    Language {
        locale: "he_IL",
        name: "Hebrew (Israel)",
        native: "עברית",
        flag: "flag-il",
    },
    Language {
        locale: "hi_IN",
        name: "Hindi (India)",
        native: "हिन्दी",
        flag: "flag-in",
    },
    Language {
        locale: "th_TH",
        name: "Thai (Thailand)",
        native: "ไทย",
        flag: "flag-th",
    },
    Language {
        locale: "vi_VN",
        name: "Vietnamese (Vietnam)",
        native: "Tiếng Việt",
        flag: "flag-vn",
    },
    Language {
        locale: "id_ID",
        name: "Indonesian (Indonesia)",
        native: "Bahasa Indonesia",
        flag: "flag-id",
    },
    Language {
        locale: "ms_MY",
        name: "Malay (Malaysia)",
        native: "Bahasa Malaysia",
        flag: "flag-my",
    },
    Language {
        locale: "tl_PH",
        name: "Filipino (Philippines)",
        native: "Filipino",
        flag: "flag-ph",
    },
    Language {
        locale: "bn_BD",
        name: "Bengali (Bangladesh)",
        native: "বাংলা",
        flag: "flag-bd",
    },
    Language {
        locale: "ta_IN",
        name: "Tamil (India)",
        native: "தமிழ்",
        flag: "flag-in",
    },
    Language {
        locale: "te_IN",
        name: "Telugu (India)",
        native: "తెలుగు",
        flag: "flag-in",
    },
    Language {
        locale: "mr_IN",
        name: "Marathi (India)",
        native: "मराठी",
        flag: "flag-in",
    },
    Language {
        locale: "ur_PK",
        name: "Urdu (Pakistan)",
        native: "اردو",
        flag: "flag-pk",
    },
    Language {
        locale: "fa_IR",
        name: "Persian (Iran)",
        native: "فارسی",
        flag: "flag-ir",
    },
    Language {
        locale: "az_AZ",
        name: "Azerbaijani (Azerbaijan)",
        native: "Azərbaycan",
        flag: "flag-az",
    },
    Language {
        locale: "kk_KZ",
        name: "Kazakh (Kazakhstan)",
        native: "Қазақша",
        flag: "flag-kz",
    },
    Language {
        locale: "uz_UZ",
        name: "Uzbek (Uzbekistan)",
        native: "Oʻzbekcha",
        flag: "flag-uz",
    },
    Language {
        locale: "mn_MN",
        name: "Mongolian (Mongolia)",
        native: "Монгол",
        flag: "flag-mn",
    },
    Language {
        locale: "sw_KE",
        name: "Swahili (Kenya)",
        native: "Kiswahili",
        flag: "flag-ke",
    },
    Language {
        locale: "af_ZA",
        name: "Afrikaans (South Africa)",
        native: "Afrikaans",
        flag: "flag-za",
    },
    Language {
        locale: "eo_EO",
        name: "Esperanto",
        native: "Esperanto",
        flag: "flag-eo",
    },
];

/// Find the index of the language whose locale matches the given string.
pub fn find_language(locale: &str) -> Option<usize> {
    LANGUAGES.iter().position(|lang| lang.locale == locale)
}

/// Find the flag icon name for a given locale.
pub fn flag_icon(locale: &str) -> &'static str {
    LANGUAGES
        .iter()
        .find(|lang| lang.locale == locale)
        .map(|lang| lang.flag)
        .unwrap_or("flag-us")
}

/// Find the native display name for a given locale.
pub fn native_name(locale: &str) -> &'static str {
    LANGUAGES
        .iter()
        .find(|lang| lang.locale == locale)
        .map(|lang| lang.native)
        .unwrap_or("English")
}

/// Directory containing JSON translation files.
const I18N_DIR: &str = "/usr/share/man-utilities/i18n";

/// Path to the file that stores the user's language preference.
const LANG_FILE: &str = "/var/lib/man/man-language";

struct Translator {
    translations: HashMap<String, String>,
    locale: String,
}

static TRANSLATOR: OnceLock<Translator> = OnceLock::new();

// English fallback strings for every translatable key.
const ENGLISH: &[(&str, &str)] = &[
    ("app_title", "MAN Utilities"),
    ("subtitle", "MAN Recovery & Installation Environment"),
    ("install_man", "Install MAN"),
    ("terminal", "Terminal"),
    ("diskulator", "Diskulator"),
    ("set_up", "Set up a new MAN system"),
    (
        "set_up_desc",
        "The assistant will review the license, select a disk, erase it, and install a bootable copy of MAN.",
    ),
    ("back", "Back"),
    ("continue", "Continue"),
    ("license_title", "Software License Agreement"),
    ("agree", "I Agree"),
    ("disagree", "I Disagree"),
    ("select_disk", "Select Disk"),
    ("installing", "Installing MAN…"),
    ("installing_on_disk", "MAN will be installed on the disk"),
    ("rebooting", "Rebooting MAN…"),
    ("language", "Language"),
    ("language_prompt", "Choose your language"),
    ("search", "Search languages…"),
    ("ok", "OK"),
    ("cancel", "Cancel"),
    ("apply", "Apply"),
    ("retry", "Retry"),
    ("done", "Done"),
    ("error", "Error"),
    ("welcome", "Welcome to MAN"),
    ("welcome_subtitle", "Select an action to begin."),
    ("partition_man", "Partition for MAN"),
    ("erase_fat32", "Erase as FAT32"),
    ("erase_ext2", "Erase as ext2"),
    ("first_aid", "Run First Aid"),
    (
        "partition_man_desc",
        "Create a GPT, 256 MiB EFI System Partition, and an ext2 data partition.",
    ),
    (
        "erase_fat32_desc",
        "Create one MBR FAT32 partition for maximum device compatibility.",
    ),
    (
        "erase_ext2_desc",
        "Create one MBR ext2 partition for Linux storage.",
    ),
    (
        "first_aid_desc",
        "Check every recognized filesystem on this disk without modifying it.",
    ),
    (
        "no_unmounted_disks",
        "No unmounted installation disks were found.",
    ),
    ("choose_disk", "Choose a Disk"),
    ("removable", "Removable"),
    ("erase_install", "Erase & Install"),
    ("confirm_installation", "Confirm Installation"),
    (
        "erase_warning",
        "This operation permanently erases the partition table and data on the selected disk.",
    ),
    ("erase_confirm_heading", "Erase this disk and install MAN?"),
    (
        "erase_destroy_all",
        "All partitions, operating systems, applications, and files on this disk will be permanently deleted.",
    ),
    (
        "first_aid_warning",
        "First Aid performs read-only filesystem checks.",
    ),
    ("selected_disk", "selected disk"),
    ("about_minutes", "About {0} minutes remaining"),
    ("cancel", "Cancel"),
    ("install_complete", "MAN was installed successfully"),
    (
        "install_complete_desc",
        "Remove the installer media. MAN will restart into the newly installed system.",
    ),
    ("rebooting_in", "Rebooting automatically in {0} seconds"),
    ("reboot_now", "Reboot Now"),
    ("operation_stopped", "The operation did not complete"),
    ("return_to_utilities", "Return to Utilities"),
    ("devices", "Devices"),
    ("storage_sub", "Internal & removable storage"),
    ("no_disks", "No physical disks detected."),
    ("refresh", "Refresh Devices"),
    ("capacity_map", "Capacity Map"),
    ("partitioned", "Partitioned"),
    ("unallocated", "Unallocated"),
    ("partition_count", "Partitions"),
    ("volumes_partitions", "Volumes & Partitions"),
    ("no_partition_table", "No partition table detected"),
    (
        "init_disk",
        "Use Partition for MAN or an erase action to initialize this disk.",
    ),
    ("filesystem_colon", "Filesystem:"),
    ("mount_colon", "Mount:"),
    ("in_use", "In use"),
    ("ready", "Ready"),
    ("internal_disk", "Internal disk"),
    ("removable_disk", "Removable disk"),
    (
        "mounted_warning",
        "This disk is mounted. Destructive actions are disabled until it is unmounted.",
    ),
    (
        "choose_operation",
        "Choose an operation above. Diskulator will confirm before changing data.",
    ),
    ("working", "Working"),
    ("disk_op_complete", "Disk operation completed"),
    ("disk_confirm", "Confirm Disk Operation"),
    ("done", "Done"),
    (
        "operation_cancelled",
        "Operation cancelled. The destination may be incomplete.",
    ),
    ("rebooting", "Rebooting MAN…"),
    ("preparing_dest", "Preparing the destination…"),
    ("preparing_disk", "Preparing Diskulator…"),
    ("select_disk_subtitle", "Select the destination for MAN"),
    ("disk_path", "Disk"),
    ("disk_size", "Size"),
    (
        "no_disks_desc",
        "Choose a storage device from the sidebar to inspect or repair it.",
    ),
    (
        "select_disk_desc",
        "Choose a storage device from the sidebar to inspect or repair it.",
    ),
    ("filesystem_mount", "Filesystem: {fs} · Mount: {mount}"),
    // OOBE (Onboarding / First-Start) strings — fully translatable.
    ("oobe_setup", "MAN Setup "),
    ("oobe_welcome", "Welcome to MAN"),
    ("oobe_welcome_to", "Welcome"),
    ("oobe_make_yours", "Let’s make this MAN yours"),
    (
        "oobe_full_tour",
        "The full tour covers language, input, appearance, accessibility, privacy, updates, navigation, and favorite apps.",
    ),
    ("oobe_start_full", "Start Full Setup"),
    ("oobe_skip_personal", "Skip OOBE Personalization"),
    (
        "oobe_quick_note",
        "The quick path still requires accepting the license and creating your local account.",
    ),
    ("oobe_license", "Please review the terms that apply to MAN."),
    ("oobe_agree", "I agree to the MAN license terms"),
    ("oobe_full_name", "Your name"),
    ("oobe_account_name", "Account name"),
    ("oobe_password", "Password"),
    ("oobe_confirm_password", "Confirm password"),
    (
        "oobe_password_note",
        "Use at least six characters. Your password stays on this computer.",
    ),
    ("oobe_passwords_mismatch", "Passwords do not match."),
    ("oobe_language", "Language"),
    ("oobe_language_short", "Language & region"),
    ("oobe_region", "Region"),
    ("oobe_region_short", "Region & Formats"),
    ("oobe_keyboard", "Keyboard"),
    ("oobe_theme", "Choose a Theme"),
    ("oobe_theme_short", "Theme"),
    ("oobe_accent", "Accent Color"),
    ("oobe_wallpaper", "Desktop Background"),
    ("oobe_layout", "Desktop Layout"),
    ("oobe_layout_short", "Layout"),
    ("oobe_dock", "Dock Behavior"),
    ("oobe_dock_short", "Dock"),
    ("oobe_accessibility", "Accessibility"),
    ("oobe_large_text", "Large text"),
    ("oobe_large_text_desc", "Increase interface text size"),
    ("oobe_high_contrast", "High contrast"),
    ("oobe_high_contrast_desc", "Strengthen visual separation"),
    ("oobe_reduce_motion", "Reduce motion"),
    ("oobe_reduce_motion_desc", "Use fewer animations"),
    ("oobe_screen_reader", "Screen reader"),
    ("oobe_screen_reader_desc", "Read controls and text aloud"),
    ("oobe_privacy", "Privacy"),
    ("oobe_location", "Location services"),
    ("oobe_location_desc", "Allow apps to request your location"),
    ("oobe_diagnostics", "Anonymous diagnostics"),
    (
        "oobe_diagnostics_desc",
        "Help find crashes without personal content",
    ),
    ("oobe_updates", "Updates"),
    ("oobe_auto_updates", "Automatic security updates"),
    (
        "oobe_auto_updates_desc",
        "Download important fixes in the background",
    ),
    ("oobe_timezone", "Date & Time"),
    ("oobe_timezone_short", "Time zone"),
    ("oobe_gestures", "Gestures & Navigation"),
    ("oobe_gestures_short", "Gestures"),
    ("oobe_touchpad_gestures", "Touchpad gestures"),
    ("oobe_touchpad_gestures_desc", "Swipe between workspaces"),
    ("oobe_tiling", "Tiling by default"),
    ("oobe_tiling_desc", "Arrange new windows automatically"),
    ("oobe_apps", "Favorite Applications"),
    ("oobe_terminal", "Terminal"),
    ("oobe_terminal_desc", "Pin Terminal to the dock"),
    ("oobe_files", "Files"),
    ("oobe_files_desc", "Pin Files to the dock"),
    ("oobe_settings", "Settings"),
    ("oobe_settings_desc", "Pin Settings to the dock"),
    ("oobe_review", "Ready for MAN"),
    (
        "oobe_review_sub",
        "Review your choices before the desktop opens.",
    ),
    ("oobe_account", "Account"),
    ("oobe_personalization", "Personalization"),
    ("oobe_skipped", "Skipped — MAN defaults"),
    ("oobe_apply", "Apply & Enter MAN"),
    ("oobe_applying", "Creating your account…"),
    (
        "oobe_applying_sub",
        "Account files and COSMIC preferences are being written atomically.",
    ),
    ("oobe_done", "MAN Is Ready"),
    (
        "oobe_done_sub",
        "Your account and desktop preferences are ready.",
    ),
    (
        "oobe_change_later",
        "You can change every personalization choice later in Settings.",
    ),
    ("oobe_enter_desktop", "Enter MAN Desktop"),
    ("oobe_try_again", "Try Again"),
    ("oobe_error", "Setup Could Not Finish"),
    (
        "oobe_error_sub",
        "No completion marker was written; you can safely try again.",
    ),
    (
        "oobe_welcome_sub",
        "A guided first start for your new computer.",
    ),
    (
        "oobe_no_completion",
        "No completion marker was written; you can safely try again.",
    ),
    (
        "oobe_account_sub",
        "This local administrator account belongs to you.",
    ),
    (
        "oobe_applying_sub2",
        "Please keep this computer powered on.",
    ),
    (
        "oobe_make_home",
        "Make MAN feel at home from the very first session.",
    ),
    ("oobe_lang_en_us", "English (United States)"),
    ("oobe_lang_en_us_desc", "English interface"),
    ("oobe_lang_en_gb", "English (United Kingdom)"),
    ("oobe_lang_en_gb_desc", "British English"),
    ("oobe_lang_de", "Deutsch"),
    ("oobe_lang_de_desc", "Deutsche Oberfläche"),
    ("oobe_lang_cs", "Čeština"),
    ("oobe_lang_cs_desc", "České rozhraní"),
    ("oobe_region_us", "United States"),
    ("oobe_region_us_desc", "12-hour time · US formats"),
    ("oobe_region_uk", "United Kingdom"),
    ("oobe_region_uk_desc", "24-hour time · UK formats"),
    ("oobe_region_cz", "Czech Republic"),
    ("oobe_region_cz_desc", "24-hour time · metric"),
    ("oobe_region_de", "Germany"),
    ("oobe_region_de_desc", "24-hour time · metric"),
    ("oobe_kb_us", "US English"),
    ("oobe_kb_us_desc", "QWERTY"),
    ("oobe_kb_uk", "UK English"),
    ("oobe_kb_uk_desc", "QWERTY"),
    ("oobe_kb_cz", "Czech"),
    ("oobe_kb_cz_desc", "QWERTZ"),
    ("oobe_kb_de", "German"),
    ("oobe_kb_de_desc", "QWERTZ"),
    ("oobe_theme_light", "Light"),
    ("oobe_theme_light_desc", "Bright surfaces and dark text"),
    ("oobe_theme_dark", "Dark"),
    ("oobe_theme_dark_desc", "Dim surfaces and light text"),
    ("oobe_theme_auto", "Automatic"),
    ("oobe_theme_auto_desc", "Follow sunrise and sunset"),
    ("oobe_accent_blue", "Cosmic Blue"),
    ("oobe_accent_blue_desc", "Calm and clear"),
    ("oobe_accent_orange", "MAN Orange"),
    ("oobe_accent_orange_desc", "Warm and energetic"),
    ("oobe_accent_green", "Forest Green"),
    ("oobe_accent_green_desc", "Natural and focused"),
    ("oobe_accent_violet", "Violet"),
    ("oobe_accent_violet_desc", "Creative and vivid"),
    ("oobe_layout_balanced", "Balanced"),
    ("oobe_layout_balanced_desc", "Panel above, dock below"),
    ("oobe_layout_compact", "Compact"),
    ("oobe_layout_compact_desc", "More room for your work"),
    ("oobe_layout_focused", "Focused"),
    ("oobe_layout_focused_desc", "Hidden controls until needed"),
    ("oobe_dock_auto", "Intelligent Hide"),
    ("oobe_dock_auto_desc", "Hide when a window overlaps"),
    ("oobe_dock_visible", "Always Visible"),
    ("oobe_dock_visible_desc", "Keep favorites in view"),
    ("oobe_dock_hidden", "Always Hidden"),
    ("oobe_dock_hidden_desc", "Reveal at the screen edge"),
    ("oobe_language_default", "English (United States)"),
    ("oobe_region_default", "United States"),
    ("oobe_keyboard_default", "US English"),
    ("oobe_theme_default", "Dark"),
    ("oobe_accent_default", "Cosmic Blue"),
    ("oobe_layout_default", "Balanced"),
    ("oobe_dock_default", "Intelligent Hide"),
    ("oobe_tz_prague", "Europe / Prague"),
    ("oobe_tz_prague_desc", "Central European time"),
    ("oobe_tz_london", "Europe / London"),
    ("oobe_tz_london_desc", "United Kingdom time"),
    ("oobe_tz_ny", "America / New York"),
    ("oobe_tz_ny_desc", "Eastern time"),
    ("oobe_tz_tokyo", "Asia / Tokyo"),
    ("oobe_tz_tokyo_desc", "Japan standard time"),
    ("oobe_wallpaper_default", "Orion Nebula"),
    ("oobe_wallpaper_default_desc", "Deep space in warm color"),
    ("oobe_wallpaper_earth", "Otherworldly Earth"),
    ("oobe_wallpaper_earth_desc", "Our planet from orbit"),
    ("oobe_wallpaper_moons", "Round Moons"),
    ("oobe_wallpaper_moons_desc", "Minimal lunar shapes"),
    ("oobe_wallpaper_webb", "Webb Inspired"),
    ("oobe_wallpaper_webb_desc", "A crisp cosmic field"),
    ("oobe_done_welcome", "Welcome, {0}!"),
    ("oobe_complete", "Complete"),
    ("oobe_failed", "Failed"),
    ("oobe_diagnostics_enabled", "Anonymous diagnostics enabled"),
    ("oobe_diagnostics_disabled", "Diagnostics disabled"),
    ("oobe_license_short", "License Agreement"),
    ("on_state", "On  ✓"),
    ("off_state", "Off"),
];

/// Initialise the global translator. Must be called once at application start.
pub fn init() {
    let locale = load_language();
    let translations = load_translations(&locale);
    let _ = TRANSLATOR.set(Translator {
        translations,
        locale,
    });
}

/// Translate a key, falling back to English.
pub fn tr(key: &str) -> String {
    let translator = TRANSLATOR.get();
    if let Some(t) = translator {
        if let Some(translated) = t.translations.get(key) {
            return translated.clone();
        }
    }
    // Fallback to English
    for (k, v) in ENGLISH {
        if *k == key {
            return (*v).to_string();
        }
    }
    key.to_string()
}

/// Returns the currently active locale (e.g. "en_US").
pub fn current_locale() -> String {
    TRANSLATOR
        .get()
        .map(|t| t.locale.clone())
        .unwrap_or_else(|| "en_US".to_string())
}

/// Save the selected language preference so future sessions use it.
pub fn save_language(locale: &str) {
    let _ = std::fs::create_dir_all("/var/lib/man");
    // Best-effort write — ignore errors (e.g. read-only recovery environment
    // where it will be re-prompted each boot).
    let _ = std::fs::write(LANG_FILE, locale);
}

/// Returns true when a persisted language preference already exists.
pub fn has_language_preference() -> bool {
    std::path::Path::new(LANG_FILE).exists()
}

fn load_language() -> String {
    // 1. Respect the LANG/LC_ALL environment variable if already set.
    if let Ok(lang) = std::env::var("LANG") {
        let lang = lang.trim();
        if !lang.is_empty() && lang != "C" && lang != "POSIX" {
            // Accept LANG="en_US.UTF-8" → "en_US"
            let lang = lang.split('.').next().unwrap_or(lang);
            if LANGUAGES.iter().any(|l| l.locale == lang) {
                return lang.to_string();
            }
        }
    }
    // 2. Fall back to the persisted preference.
    if let Ok(saved) = std::fs::read_to_string(LANG_FILE) {
        let saved = saved.trim();
        if !saved.is_empty() && LANGUAGES.iter().any(|l| l.locale == saved) {
            return saved.to_string();
        }
    }
    // 3. Default to US English.
    "en_US".to_string()
}

fn load_translations(locale: &str) -> HashMap<String, String> {
    let path = format!("{}/{}.json", I18N_DIR, locale);
    let mut map = HashMap::new();
    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(parsed) = serde_json::from_str::<HashMap<String, String>>(&content) {
            map = parsed;
        }
    }
    map
}
