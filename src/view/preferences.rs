use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

#[derive(Clone, Copy, Serialize, Deserialize, Default, Debug)]
pub enum Telemetry {
    #[default]
    All,
    OnlyVersionCheck,
    Nothing,
}

#[derive(Clone, Copy, Serialize, Deserialize, Default, Debug)]
pub enum DarkModePref {
    #[default]
    FollowSystem,
    Dark,
    Light,
}

#[derive(Clone, Copy, Serialize, Deserialize, Default, Debug)]
pub enum AutoDeletePref {
    NoTime,
    OneDay,
    #[default]
    OneWeek,
    OneMonth,
    Eternity,
}

#[derive(Clone, Copy, Serialize, Deserialize, Default, Debug)]
pub enum CacheSize {
    None,
    #[default]
    Normal,
    Large,
}
impl ToString for CacheSize {
    fn to_string(&self) -> String {
        match self {
            CacheSize::None => "None".to_string(),
            CacheSize::Normal => "Normal".to_string(),
            CacheSize::Large => "Large".to_string(),
        }
    }
}

impl CacheSize {
    pub fn value(self) -> usize {
        match self {
            CacheSize::None => 0,
            CacheSize::Normal => 100,
            CacheSize::Large => 10_000,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Default, EnumIter, Debug)]
pub enum Language {
    #[default]
    EN,
    DE,
    FR,
}

impl Language {
    pub fn apply(self) {
        rust_i18n::set_locale(match self {
            Language::EN => "en",
            Language::DE => "de",
            Language::FR => "fr",
        });
    }
}

impl ToString for Language {
    /// The identifier used in the locale filenames
    fn to_string(&self) -> String {
        match self {
            Language::EN => "EN".to_string(),
            Language::DE => "DE".to_string(),
            Language::FR => "FR".to_string(),
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug)]
pub struct Preferences {
    #[serde(default)]
    pub darkmode: DarkModePref,
    #[serde(default)]
    pub auto_delete: AutoDeletePref,
    #[serde(default)]
    pub cache_size: CacheSize,
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub telemetry: Telemetry,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            darkmode: DarkModePref::FollowSystem,
            auto_delete: AutoDeletePref::Eternity,
            cache_size: CacheSize::Normal,
            language: Language::EN,
            telemetry: Telemetry::All,
        }
    }
}
