use anyhow::Context;
use chrono::{DateTime, FixedOffset, Local, TimeZone};
use directories_next::ProjectDirs;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_FILENAME_STEM: &str = "de99-1970-01-01-00-00-00T00-00-00";
const FORMAT_FILENAME: &str = "%Y-%m-%d-%H-%M-%ST%::z";
const FORMAT_DISPLAY: &str = "%Y-%m-%d %H:%M:%S";

/// Takes care of persistent storage to disk. Yes, this makes it impossible
/// to run in the browser, which only provides a String-String database. But
/// on Desktop this will improve the functionality immensely
/// if we want to run this on our server some day, it'll require some rework
/// and quite a few cfg!(), but it's worth it for the local functionality

#[derive(Debug, Clone)]
pub struct SavedDB {
    pub path: PathBuf,
    pub date: DateTime<Local>,
    pub server_str: String,
}

impl From<PathBuf> for SavedDB {
    fn from(path: PathBuf) -> Self {
        let default_filename = OsString::from(DEFAULT_FILENAME_STEM);
        let filename = path
            .file_stem()
            .unwrap_or(&default_filename)
            .to_str()
            .unwrap_or(DEFAULT_FILENAME_STEM);
        let splits: Vec<&str> = filename.splitn(2, '-').collect();
        let server_str = splits[0];
        let date_str = splits[1];

        // first we need to turn "de99-1970-01-01-00-00-00T00-00-00" into "de99-1970-01-01-00-00-00T00:00:00" so chrono can parse it.
        // This shit is done for legacy reasons. I used to use the time crate, which allowed more flexible offset formatting and I decided to use - instead of : for separating the offset code.
        let rev: String = date_str.chars().rev().collect();
        let swapped = rev.replacen('-', ":", 2);
        let date_str: String = swapped.chars().rev().collect();

        let date_fixed = DateTime::parse_from_str(&date_str, FORMAT_FILENAME).unwrap_or(
            FixedOffset::east_opt(0)
                .unwrap()
                .with_ymd_and_hms(1970, 01, 01, 00, 00, 00)
                .unwrap(),
        );

        // not sure if that is properly localized. Technically we don't need to convert to current
        // local offset, we need to convert to local offset at the time of the date. But maybe to
        // current_local_offset is actually better? That way all times are accurate in relation to
        // now, instead of being accurate to the clock of the past (which may be in a different
        // timezone after DST change)
        let date_local: DateTime<Local> = date_fixed.with_timezone(&Local);

        Self {
            path: path.clone(),
            date: date_local,
            server_str: server_str.into(),
        }
    }
}

impl Ord for SavedDB {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.path.cmp(&other.path)
    }
}

impl PartialOrd for SavedDB {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.path.partial_cmp(&other.path)
    }
}

impl Eq for SavedDB {}
impl PartialEq for SavedDB {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl Display for SavedDB {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time_str = self.date.format(&FORMAT_DISPLAY).to_string();
        write!(f, "{time_str}")
    }
}

/// returns a path to a not yet existing apiresponse file. If the
/// function returns `Some(path)`, the parent directory is
/// guaranteed to exist.
pub fn get_new_db_filename(server: &str, now: &DateTime<Local>) -> Option<PathBuf> {
    if !ensure_storage_location_exists() {
        return None;
    }

    let dir = storage_dir()?;
    let time_str = now
        .format(FORMAT_FILENAME) // format as "YYYY-MM-DD-HH-MM-SST+HH:MM:SS"
        .to_string()
        .replace(":", "-"); // backwards compatibility: turn the above string into "YYYY-MM-DD-HH-MM-SST+HH-MM-SS"
    let filename = format!("{server}-{time_str}.apiresponse");
    Some(dir.join(filename))
}

/// get a list of all saved databases
pub fn get_list_of_saved_dbs() -> BTreeMap<String, Vec<SavedDB>> {
    let mut re = BTreeMap::new();

    // only progress if the storage dir exists
    // there is no use calling ensure_storage_location_exists here,
    // because if we create it now it will be empty anyway
    let opt_dir = storage_dir();
    if opt_dir.is_none() {
        eprintln!("did not find the storage dir");
        return re;
    }
    let dir = opt_dir.unwrap();

    // only progress if we can read the storage dir
    // TODO maybe we can tell the user what went wrong, if we can't read the directory?
    let res_files = fs::read_dir(dir);
    if res_files.is_err() {
        eprintln!("did not find any files in the storage dir");
        return re;
    }
    let files = res_files.unwrap();

    // get a list of all files that have the "sqlite" or apiresponse extension
    let db_files: Vec<SavedDB> = files
        .flatten()
        .map(|e| e.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            path.extension() == Some(OsStr::new("sqlite"))
                || path.extension() == Some(OsStr::new("apiresponse"))
        })
        .map(SavedDB::from)
        .collect();
    // push them into a BTreeMap
    for saved_db in db_files {
        re.entry(saved_db.server_str.clone())
            .or_insert(Vec::new())
            .push(saved_db);
    }

    // Sort each entry in the BTreeMap
    for saved_dbs in re.values_mut() {
        saved_dbs.sort();
    }
    re
}

/// attempts to delete the given file
pub fn remove_db(filename: &Path) -> anyhow::Result<()> {
    fs::remove_file(filename).with_context(|| format!("Failed to delete {filename:?}"))
}

pub fn remove_all() {
    for (_server, list_of_dbs) in get_list_of_saved_dbs() {
        // TODO let the use know if something can't be deleted
        for saved_db in list_of_dbs {
            let _result = remove_db(saved_db.path.as_path());
        }
    }
}

// utility functions

fn my_project_dir() -> Option<ProjectDirs> {
    ProjectDirs::from("", "", "TurunMap")
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
