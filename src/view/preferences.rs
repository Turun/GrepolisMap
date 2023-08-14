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

#[derive(Clone, Copy)]
pub struct Preferences {
    darkmode: DarkModePref,
    auto_delete: AutoDeletePref,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            darkmode: DarkModePref::FollowSystem,
            auto_delete: AutoDeletePref::Eternity,
        }
    }
}
