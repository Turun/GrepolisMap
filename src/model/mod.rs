use crate::constraint::ConstraintType;
use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::selection::AndOr;
use crate::town::Town;
use eframe::epaint::ahash::HashMap;
use std::collections::hash_map::Entry;
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Duration;

pub(crate) mod database;
pub mod download;
mod offset_data;

const DECAY: f32 = 0.9;
const MIN_AGE: f32 = 0.1; // anything that was not touched `DECAY.powi(20)` times in a row should be removed from cache

type StringCacheKey = (
    ConstraintType,
    Vec<EmptyConstraint>,
    AndOr,
    BTreeSet<EmptyTownSelection>,
);
type TownCacheKey = (Vec<EmptyConstraint>, AndOr, BTreeSet<EmptyTownSelection>);

pub enum Model {
    Uninitialized,
    Loaded {
        db: database::DataTable,
        ctx: egui::Context,
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
            Model::Uninitialized => { /*do nothing*/ }
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

    pub fn request_repaint_after(&self, duration: Duration) {
        match self {
            Model::Uninitialized => { /*do nothing*/ }
            Model::Loaded { ctx, .. } => {
                ctx.request_repaint_after(duration);
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
            Model::Uninitialized => Ok(Arc::new(Vec::new())),
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
                        let value = Arc::new(
                            db.get_towns_for_constraints(&this_selection, all_selections)?,
                        );
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
            Model::Uninitialized => Ok(Arc::new(Vec::new())),
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
                            )?,
                            AndOr::Or => {
                                let present_constraints: Vec<&str> = constraints
                                    .iter()
                                    .filter(|c| c.constraint_type == constraint_type)
                                    .map(|c| c.value.as_str())
                                    .collect();
                                db.get_names_for_constraint_type(constraint_type)?
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

    pub fn get_ghost_towns(&self) -> anyhow::Result<Arc<Vec<Town>>> {
        match self {
            Model::Uninitialized => Ok(Arc::new(Vec::new())),
            Model::Loaded { db, .. } => Ok(Arc::new(db.get_ghost_towns()?)),
        }
    }

    pub fn get_all_towns(&self) -> anyhow::Result<Arc<Vec<Town>>> {
        match self {
            Model::Uninitialized => Ok(Arc::new(Vec::new())),
            Model::Loaded { db, .. } => Ok(Arc::new(db.get_all_towns()?)),
        }
    }

    pub fn get_names_for_constraint_type(
        &mut self,
        constraint_type: ConstraintType,
    ) -> anyhow::Result<Arc<Vec<String>>> {
        match self {
            Model::Uninitialized => Ok(Arc::new(Vec::new())),
            Model::Loaded {
                db, cache_strings, ..
            } => {
                let key = (constraint_type, Vec::new(), AndOr::And, BTreeSet::new());
                let value = match cache_strings.entry(key) {
                    Entry::Occupied(entry) => {
                        let tuple = entry.into_mut();
                        tuple.0 += 1.0;
                        tuple.1.clone()
                    }
                    Entry::Vacant(entry) => {
                        let value = Arc::new(db.get_names_for_constraint_type(constraint_type)?);
                        entry.insert((1.0, value)).1.clone()
                    }
                };
                Ok(value)
            }
        }
    }
}
