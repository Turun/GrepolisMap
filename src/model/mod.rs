use crate::constraint::ConstraintType;
use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::selection::AndOr;

#[cfg(not(target_arch = "wasm32"))]
use crate::storage::{self, SavedDB};
use crate::town::Town;
use anyhow::Context;
use eframe::epaint::ahash::HashMap;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::Entry;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{fs, thread};
#[cfg(not(target_arch = "wasm32"))]
use time::{OffsetDateTime, UtcOffset};

pub(crate) mod database;
pub mod download;
mod offset_data;
#[cfg(not(target_arch = "wasm32"))]
mod parse_sqlite;

const DECAY: f32 = 0.9;
const MIN_AGE: f32 = 0.1; // anything that was not touched `DECAY.powi(20)` times in a row should be removed from cache

type StringCacheKey = (
    ConstraintType,
    Vec<EmptyConstraint>,
    AndOr,
    BTreeSet<EmptyTownSelection>,
);
type TownCacheKey = (Vec<EmptyConstraint>, AndOr, BTreeSet<EmptyTownSelection>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIResponse {
    pub for_server: String,
    #[cfg(not(target_arch = "wasm32"))]
    pub filename: Option<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    pub timestamp: OffsetDateTime,

    players: Option<String>,
    alliances: Option<String>,
    towns: Option<String>,
    islands: Option<String>,
}

impl APIResponse {
    pub fn new(server_id: String) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
            let now = OffsetDateTime::now_utc().to_offset(local_offset);
            let filename = storage::get_new_db_filename(&server_id, &now);
            Self {
                for_server: server_id,
                filename,
                timestamp: now,
                players: None,
                alliances: None,
                towns: None,
                islands: None,
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                for_server: server_id,
                players: None,
                alliances: None,
                towns: None,
                islands: None,
            }
        }
    }

    /// how many of the fields are already populated/loaded?
    pub fn count_completed(&self) -> u8 {
        let mut re = 0;
        if self.players.is_some() {
            re += 1;
        }
        if self.alliances.is_some() {
            re += 1;
        }
        if self.towns.is_some() {
            re += 1;
        }
        if self.islands.is_some() {
            re += 1;
        }
        return re;
    }

    pub fn is_complete(&self) -> bool {
        return self.count_completed() == 4;
    }

    /// given a filepath, load the previously fetched API Response and put it into the `api_results` out variable. This is done so the UI doesn't hang.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from_file(saved_db: SavedDB, api_results: Arc<Mutex<APIResponse>>) {
        thread::spawn(move || {
            // TODO: improve error handling
            match saved_db.path.extension().and_then(|ext| ext.to_str()) {
                Some("sqlite") => {
                    let api_response = parse_sqlite::sqlite_to_apiresponse(saved_db).context("failed to parse the api response from the sqlite file saved at {path:?}")
                        .unwrap();
                    let mut guard = api_results.lock().unwrap();
                    *guard = api_response;
                }
                Some("apiresponse") => {
                    // read file content
                    let s = fs::read_to_string(saved_db.path.clone()).unwrap();
                    // convert to api response
                    let api_response = serde_json::from_str(&s)
                        .context(format!(
                            "failes to parse api response from json saved at {:?}",
                            saved_db.path
                        ))
                        .unwrap();
                    let mut guard = api_results.lock().unwrap();
                    *guard = api_response;
                }
                Some(_) | None => {
                    eprintln!("Can not load data from file {:?}", saved_db.path);
                }
            }
        });
    }

    /// save the api response to the file as defined in self.filename.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_file(&self) {
        // only relevant on native. WASM does not get to save old api responses
        let opt_output_string = serde_json::to_string(self);
        let opt_filename = self.filename.clone();
        // write to file in a different thread. Otherwise we hang the UI on slow systems.
        let _handle = thread::spawn(move || {
            match (opt_filename, opt_output_string) {
                (None, Ok(_output_string)) => {
                    eprintln!("no filename to save the api response to");
                }
                (Some(filename), Ok(output_string)) => {
                    let msg = format!("failed to write api resonse to file ({filename:?}):");
                    if filename.exists() {
                        println!(
                            "skip saving api response to file, because the file exists already."
                        );
                    } else {
                        match fs::write(filename, output_string) {
                            Ok(()) => {
                                println!("successfully saved api response to file");
                            }
                            Err(err) => {
                                eprintln!("{msg}\n{err:?}");
                            }
                        }
                    }
                }
                (None, Err(err)) => {
                    eprintln!("no filename to save the api response to");
                    eprintln!("failed to convert the api response to a json string: {err:?}");
                }
                (Some(_filename), Err(err)) => {
                    eprintln!("failed to convert the api response to a json string: {err:?}");
                }
            };
        });
    }
}

pub enum Model {
    Uninitialized(Arc<Mutex<APIResponse>>),
    Loaded {
        db: database::DataTable,
        cache_strings: HashMap<StringCacheKey, (f32, Arc<Vec<String>>)>,
        cache_towns: HashMap<TownCacheKey, (f32, Arc<Vec<Town>>)>,
    },
}

fn age_and_filter_hashmap<K, V>(map: &mut HashMap<K, (f32, V)>, keep_count: usize) {
    // reduce the age (exponential decay)
    let mut ages = map
        .values_mut()
        .map(|(age, _value)| {
            *age *= DECAY;
            *age
        })
        .collect::<Vec<f32>>();
    let cutoff = if ages.len() > keep_count {
        ages.sort_unstable_by(f32::total_cmp);
        ages[keep_count]
    } else {
        0.0
    };
    let cutoff = f32::max(cutoff, MIN_AGE);
    map.retain(|_key, (age, _value)| *age > cutoff);

    // println!(
    //     "Filter entries: Previous {}, Goal {}, Now {}; Age: max {}, cutoff {}, min {}",
    //     ages.len(),
    //     keep_count,
    //     map.len(),
    //     ages.iter().copied().reduce(f32::max).unwrap_or(f32::NAN),
    //     cutoff,
    //     ages.iter().copied().reduce(f32::min).unwrap_or(f32::NAN)
    // );
}

impl Model {
    pub fn age_cache(&mut self, keep_count: usize) {
        match self {
            Model::Uninitialized(_) => { /*do nothing*/ }
            Model::Loaded {
                cache_strings,
                cache_towns,
                ..
            } => {
                // Alternatives to the current aging method could incorporate something between LeastRecentlyUsed cache, time base cache and LeastOftenUsed cache.
                // print!("Strings: ");
                age_and_filter_hashmap(cache_strings, keep_count);
                // print!("Towns  : ");
                age_and_filter_hashmap(cache_towns, keep_count);
            }
        }
    }

    pub fn get_towns_for_constraints(
        &mut self,
        selection: &EmptyTownSelection,
        constraints: &[EmptyConstraint],
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Arc<Vec<Town>>> {
        match self {
            Model::Uninitialized(_) => Ok(Arc::new(Vec::new())),
            Model::Loaded {
                db, cache_towns, ..
            } => {
                let mut this_selection = selection.clone();
                this_selection.constraints = constraints.to_vec();

                let referenced_selections =
                    this_selection.all_referenced_selections(all_selections)?;

                let key = (
                    constraints.to_vec(),
                    selection.constraint_join_mode,
                    referenced_selections,
                );
                let value = match cache_towns.entry(key) {
                    Entry::Occupied(entry) => {
                        let tuple = entry.into_mut();
                        tuple.0 += 1.0;
                        tuple.1.clone()
                    }
                    Entry::Vacant(entry) => {
                        let value =
                            Arc::new(db.get_towns_for_constraints(&this_selection, all_selections));
                        entry.insert((1.0, value)).1.clone()
                    }
                };
                Ok(value)
            }
        }
    }

    /// get a list of all values in the DB for the given `constraint_type` which match the
    /// given `constraints`. For a selection that is joined with `AndOr::Or` this is the same as
    /// `self.get_names_for_constraint_type`. For selections joined with `AndOr::And` this is said
    /// list, but then filtered by the given `constraints`.  This function is used to provide a list
    /// of values in the drop down field for the user.
    pub fn get_names_for_constraint_with_constraints(
        &mut self,
        selection: &EmptyTownSelection,
        constraint_type: ConstraintType,
        constraints: &[EmptyConstraint],
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Arc<Vec<String>>> {
        match self {
            Model::Uninitialized(_) => Ok(Arc::new(Vec::new())),
            Model::Loaded {
                db, cache_strings, ..
            } => {
                let mut this_selection = selection.clone();
                this_selection.constraints = constraints.to_vec();

                let referenced_selections =
                    this_selection.all_referenced_selections(all_selections)?;

                let key = (
                    constraint_type,
                    constraints.to_vec(),
                    selection.constraint_join_mode,
                    referenced_selections,
                );
                let value = match cache_strings.entry(key) {
                    Entry::Occupied(entry) => {
                        let tuple = entry.into_mut();
                        tuple.0 += 1.0;
                        tuple.1.clone()
                    }
                    Entry::Vacant(entry) => {
                        let value = Arc::new(match selection.constraint_join_mode {
                            AndOr::And => db.get_names_for_constraint_type_in_constraints(
                                constraint_type,
                                &this_selection,
                                all_selections,
                            ),
                            AndOr::Or => {
                                let present_constraints: Vec<&str> = constraints
                                    .iter()
                                    .filter(|c| c.constraint_type == constraint_type)
                                    .map(|c| c.value.as_str())
                                    .collect();
                                db.get_names_for_constraint_type(constraint_type)
                                    .into_iter()
                                    .filter(|s| !present_constraints.contains(&s.as_str()))
                                    .collect()
                            }
                        });
                        entry.insert((1.0, value)).1.clone()
                    }
                };
                Ok(value)
            }
        }
    }

    pub fn get_ghost_towns(&self) -> Arc<Vec<Town>> {
        match self {
            Model::Uninitialized(_) => Arc::new(Vec::new()),
            Model::Loaded { db, .. } => Arc::new(db.get_ghost_towns()),
        }
    }

    pub fn get_all_towns(&self) -> Arc<Vec<Town>> {
        match self {
            Model::Uninitialized(_) => Arc::new(Vec::new()),
            Model::Loaded { db, .. } => Arc::new(db.get_all_towns()),
        }
    }
}
