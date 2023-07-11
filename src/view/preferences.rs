use std::fmt::Display;

#[derive(Clone, Copy)]
enum DarkModePref {
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

#[derive(Clone, Copy)]
enum AutoDeletePref {
    OneDay,
    OneWeek,
    OneMonth,
    Never,
}

impl Display for AutoDeletePref {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutoDeletePref::OneDay => write!(f, "One Day"),
            AutoDeletePref::OneWeek => write!(f, "One Week"),
            AutoDeletePref::OneMonth => write!(f, "One Month"),
            AutoDeletePref::Never => write!(f, "Never"),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Preferences {
    darkmode: DarkModePref,
    auto_delete: AutoDeletePref,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            darkmode: DarkModePref::FollowSystem,
            auto_delete: AutoDeletePref::Never,
        }
    }
}
