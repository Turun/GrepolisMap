use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum DarkModePref {
    FollowSystem,
    Dark,
    Light,
}

impl Display for DarkModePref {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DarkModePref::FollowSystem => write!(f, "Follow System Theme"),
            DarkModePref::Dark => write!(f, "Dark"),
            DarkModePref::Light => write!(f, "Light"),
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum AutoDeletePref {
    NoTime,
    OneDay,
    OneWeek,
    OneMonth,
    Eternity,
}

impl Display for AutoDeletePref {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // TODO we should try to not store any DB on the disk if the user selects "No Time"
        match self {
            AutoDeletePref::NoTime => write!(f, "No Time"),
            AutoDeletePref::OneDay => write!(f, "One Day"),
            AutoDeletePref::OneWeek => write!(f, "One Week"),
            AutoDeletePref::OneMonth => write!(f, "One Month"),
            AutoDeletePref::Eternity => write!(f, "Eternity"),
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum CacheSize {
    None,
    Normal,
    Large,
}
impl Display for CacheSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CacheSize::None => write!(f, "None"),
            CacheSize::Normal => write!(f, "Normal"),
            CacheSize::Large => write!(f, "Large"),
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

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Preferences {
    pub darkmode: DarkModePref,
    pub auto_delete: AutoDeletePref,
    pub cache_size: CacheSize,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            darkmode: DarkModePref::FollowSystem,
            auto_delete: AutoDeletePref::Eternity,
            cache_size: CacheSize::Normal,
        }
    }
}
