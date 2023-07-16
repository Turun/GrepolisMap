use crate::towns::{Constraint, ConstraintType, Town};
use eframe::epaint::ahash::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::Duration;

pub(crate) mod database;
pub mod download;
mod offset_data;

const DECAY: f32 = 0.9;
const MIN_AGE: f32 = 0.08; // anything that was not touched `DECAY.powi(25)` times in a row should be removed from cache
pub const CACHE_SIZE_NONE: usize = 0;
pub const CACHE_SIZE_SMALL: usize = 25;
pub const CACHE_SIZE_NORMAL: usize = 100;
pub const CACHE_SIZE_LARGE: usize = 1000;

pub enum Model {
    Uninitialized,
    Loaded {
        db: database::Database,
        ctx: egui::Context,
        #[allow(clippy::type_complexity)]
        cache_strings: HashMap<(ConstraintType, Vec<Constraint>), (f32, Arc<Vec<String>>)>,
        cache_towns: HashMap<Vec<Constraint>, (f32, Arc<Vec<Town>>)>,
    },
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

                // progress the age counter and remove the lowest `keep_count` values
                let mut ages = cache_strings
                    .values_mut()
                    .map(|(age, _value)| {
                        *age *= DECAY;
                        *age
                    })
                    .collect::<Vec<f32>>();
                if ages.len() > keep_count {
                    ages.sort_unstable_by(f32::total_cmp);
                    let cutoff = ages[keep_count];
                    cache_strings.retain(|_key, (age, _value)| *age >= cutoff && *age > MIN_AGE);

                    println!(
                        "Reduce String Cache from {} to {} entries, min age: {}, cutoff age {}, max age: {}" ,
                        ages.len(),
                        keep_count,
                        ages.iter().copied().reduce(f32::max).unwrap_or(f32::NAN),
                        cutoff,
                        ages.iter().copied().reduce(f32::min).unwrap_or(f32::NAN)
                    );
                } else {
                    println!(
                        "String Cache kept at {} entries, max age: {}, min age: {}",
                        ages.len(),
                        ages.iter().copied().reduce(f32::max).unwrap_or(f32::NAN),
                        ages.iter().copied().reduce(f32::min).unwrap_or(f32::NAN)
                    );
                }

                // do the same for the `cache_towns` map
                let mut ages = cache_towns
                    .values_mut()
                    .map(|(age, _value)| {
                        *age *= DECAY;
                        *age
                    })
                    .collect::<Vec<f32>>();
                if ages.len() > keep_count {
                    ages.sort_unstable_by(f32::total_cmp);
                    let cutoff = ages[keep_count];
                    cache_towns.retain(|_key, (age, _value)| *age >= cutoff && *age > MIN_AGE);

                    println!(
                        "Reduce Town Cache from {} to {} entries, min age: {}, cutoff age {}, max age: {}" ,
                        ages.len(),
                        keep_count,
                        ages.iter().copied().reduce(f32::max).unwrap_or(f32::NAN),
                        cutoff,
                        ages.iter().copied().reduce(f32::min).unwrap_or(f32::NAN)
                    );
                } else {
                    println!(
                        "Town Cache kept at {} entries, max age: {}, min age: {}",
                        ages.len(),
                        ages.iter().copied().reduce(f32::max).unwrap_or(f32::NAN),
                        ages.iter().copied().reduce(f32::min).unwrap_or(f32::NAN)
                    );
                }
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
        constraints: &[Constraint],
    ) -> anyhow::Result<Arc<Vec<Town>>> {
        match self {
            Model::Uninitialized => Ok(Arc::new(Vec::new())),
            Model::Loaded {
                db, cache_towns, ..
            } => {
                let key = constraints.to_vec();
                let value = match cache_towns.entry(key) {
                    Entry::Occupied(entry) => entry.get().1.clone(),
                    Entry::Vacant(entry) => {
                        let value = Arc::new(db.get_towns_for_constraints(constraints)?);
                        entry.insert((1.0, value)).1.clone()
                    }
                };
                Ok(value)
            }
        }
    }

    pub fn get_names_for_constraint_type_with_constraints(
        &mut self,
        constraint_type: ConstraintType,
        constraints: &[Constraint],
    ) -> anyhow::Result<Arc<Vec<String>>> {
        match self {
            Model::Uninitialized => Ok(Arc::new(Vec::new())),
            Model::Loaded {
                db, cache_strings, ..
            } => {
                let key = (constraint_type, constraints.to_vec());
                let value = match cache_strings.entry(key) {
                    Entry::Occupied(entry) => entry.get().1.clone(),
                    Entry::Vacant(entry) => {
                        let value = Arc::new(db.get_names_for_constraint_type_in_constraints(
                            constraint_type,
                            constraints,
                        )?);
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
                let key = (constraint_type, Vec::new());
                let value = match cache_strings.entry(key) {
                    Entry::Occupied(entry) => entry.get().1.clone(),
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
