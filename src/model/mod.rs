use crate::towns::{Constraint, ConstraintType, Town};
use eframe::epaint::ahash::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::Duration;

pub(crate) mod database;
pub mod download;
mod offset_data;

const DECAY: f32 = 0.9;

// keep track how well our cache is utilized
pub struct CacheCounter {
    pub hit: u32,
    pub mis: u32,
}

pub enum Model {
    Uninitialized,
    Loaded {
        db: database::Database,
        ctx: egui::Context,
        cache_strings: HashMap<(ConstraintType, Vec<Constraint>), (f32, Arc<Vec<String>>)>,
        cache_towns: HashMap<Vec<Constraint>, (f32, Arc<Vec<Town>>)>,
        cache_counter: CacheCounter,
    },
}

impl Model {
    pub fn age_cache(&mut self, keep_count: usize) {
        match self {
            Model::Uninitialized => { /*do nothing*/ }
            Model::Loaded {
                db: _,
                ctx: _,
                cache_strings,
                cache_towns,
                cache_counter,
            } => {
                // TODO the cache can grow pretty big and easily take up a few gigs of RAM if a user keeps the program running for a while.
                //   we need to delete some cache entries every now and then. Something between LeastRecentlyUsed cache, time base cache and LeastOftenUsed cache.
                //   An easy way would be to save a hit counter with every element in the cache. When the cache grows too large we can go through and remove the
                //   lower half of elements, sorted by hit count (least often used elements get eviced)
                let num_towns: usize = cache_towns
                    .values()
                    .map(|(_age, town_list)| town_list.len())
                    .sum();
                let num_strings: usize = cache_strings
                    .values()
                    .map(|(_age, str_list)| str_list.len())
                    .sum();
                let len_strings: usize = cache_strings
                    .values()
                    .flat_map(|(_age, str_list)| str_list.iter().map(std::string::String::len))
                    .sum();
                println!(
                    "hit: {}, mis: {}, total: {}, hit fraction: {}, towns: {}, strings: {}, chars: {}",
                    cache_counter.hit,
                    cache_counter.mis,
                    cache_counter.hit + cache_counter.mis,
                    f64::from(cache_counter.hit) / f64::from(cache_counter.hit + cache_counter.mis),
                    num_towns,
                    num_strings,
                    len_strings,
                );

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
                    cache_strings.retain(|_key, (age, _value)| *age >= cutoff);
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
                    cache_towns.retain(|_key, (age, _value)| *age >= cutoff);
                }
            }
        }
    }

    pub fn request_repaint_after(&self, duration: Duration) {
        match self {
            Model::Uninitialized => { /*do nothing*/ }
            Model::Loaded {
                db: _,
                ctx,
                cache_strings: _,
                cache_towns: _,
                cache_counter: _,
            } => {
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
                db,
                ctx: _ctx,
                cache_strings: _,
                cache_towns,
                cache_counter,
            } => {
                let key = constraints.to_vec();
                let value = match cache_towns.entry(key) {
                    Entry::Occupied(entry) => {
                        cache_counter.hit += 1;
                        entry.get().1.clone()
                    }
                    Entry::Vacant(entry) => {
                        cache_counter.mis += 1;
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
                db,
                ctx: _ctx,
                cache_strings,
                cache_towns: _,
                cache_counter,
            } => {
                let key = (constraint_type, constraints.to_vec());
                let value = match cache_strings.entry(key) {
                    Entry::Occupied(entry) => {
                        cache_counter.hit += 1;
                        entry.get().1.clone()
                    }
                    Entry::Vacant(entry) => {
                        cache_counter.mis += 1;
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
            Model::Loaded {
                db,
                ctx: _ctx,
                cache_strings: _,
                cache_towns: _,
                cache_counter: _,
            } => Ok(Arc::new(db.get_ghost_towns()?)),
        }
    }

    pub fn get_all_towns(&self) -> anyhow::Result<Arc<Vec<Town>>> {
        match self {
            Model::Uninitialized => Ok(Arc::new(Vec::new())),
            Model::Loaded {
                db,
                ctx: _ctx,
                cache_strings: _,
                cache_towns: _,
                cache_counter: _,
            } => Ok(Arc::new(db.get_all_towns()?)),
        }
    }

    pub fn get_names_for_constraint_type(
        &mut self,
        constraint_type: ConstraintType,
    ) -> anyhow::Result<Arc<Vec<String>>> {
        match self {
            Model::Uninitialized => Ok(Arc::new(Vec::new())),
            Model::Loaded {
                db,
                ctx: _ctx,
                cache_strings,
                cache_towns: _,
                cache_counter,
            } => {
                let key = (constraint_type, Vec::new());
                let value = match cache_strings.entry(key) {
                    Entry::Occupied(entry) => {
                        cache_counter.hit += 1;
                        entry.get().1.clone()
                    }
                    Entry::Vacant(entry) => {
                        cache_counter.mis += 1;
                        let value = Arc::new(db.get_names_for_constraint_type(constraint_type)?);
                        entry.insert((1.0, value)).1.clone()
                    }
                };
                Ok(value)
            }
        }
    }
}
