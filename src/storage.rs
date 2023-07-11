use anyhow::Context;
use directories_next::ProjectDirs;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use time::macros::format_description;
use time::OffsetDateTime;

/// Takes care of persistent storage to disk. Yes, this makes it impossible
/// to run in the browser, which only provides a String-String database. But
/// on Desktop this will improve the functionality immensely

/// returns a path to a not yet existing sqlite file. If the
/// function returns `Some(path)`, the parent directory is
/// guaranteed to exist.
pub fn get_new_db_filename(server: &str) -> Option<PathBuf> {
    if !ensure_storage_location_exists() {
        return None;
    }

    let dir = storage_dir()?;
    let format = format_description!("[year]-[month]-[day]-[hour]-[minute]-[second]UTC");
    let now = OffsetDateTime::now_utc();
    let time_str = now.format(&format).ok()?;
    let filename = format!("{server}-{time_str}.sqlite");
    Some(dir.join(filename))
}

/// get a list of all saved databases
pub fn get_list_of_saved_dbs() -> Vec<PathBuf> {
    let re = Vec::new();

    // only progress if the storage dir exists
    // there is no use calling ensure_storage_location_exists here,
    // because if we create it now it will be empty anyway
    let opt_dir = storage_dir();
    if opt_dir.is_none() {
        return re;
    }
    let dir = opt_dir.unwrap();

    // only progress if we can read the storage dir
    // TODO maybe we can tell the user what went wrong, if we can't read the directory?
    let res_files = fs::read_dir(dir);
    if res_files.is_err() {
        return re;
    }
    let files = res_files.unwrap();

    // return a list of all files that have the "sqlite" extension
    let mut db_files: Vec<PathBuf> = files
        .flatten()
        .map(|e| e.path())
        .filter(|path| path.is_file())
        .filter(|path| path.extension() == Some(OsStr::new("sqlite")))
        .collect();
    db_files.sort();
    db_files
}

/// attempts to delete the given file
pub fn remove_db(filename: &Path) -> anyhow::Result<()> {
    fs::remove_file(filename).with_context(|| format!("Failed to delete {filename:?}"))
}
}

// utility functions

fn my_project_dir() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "TurunMap")
}

fn storage_exists() -> bool {
    my_project_dir().is_some()
}

fn storage_dir() -> Option<PathBuf> {
    my_project_dir().map(|dir| dir.data_local_dir().into())
}

fn ensure_storage_location_exists() -> bool {
    if let Some(dir) = storage_dir() {
        fs::create_dir_all(dir).is_ok()
    } else {
        false
    }
}
