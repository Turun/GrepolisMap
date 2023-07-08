use crate::towns::{Constraint, ConstraintType, Town};
use eframe::epaint::ahash::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::Duration;

pub mod download;
mod offset_data;

// keep track how well our cache is utilized
pub struct CacheCounter {
    pub hit: u32,
    pub mis: u32,
}

pub enum Model {
    Uninitialized,
    Loaded {
        db: download::Database,
        ctx: egui::Context,
        cache_strings: HashMap<(ConstraintType, Vec<Constraint>), Arc<Vec<String>>>,
        cache_towns: HashMap<Vec<Constraint>, Arc<Vec<Town>>>,
        cache_counter: CacheCounter,
    },
}

impl Model {
    pub fn request_repaint_after(&self, duration: Duration) {
        match self {
            Model::Uninitialized => { /*do nothing*/ }
            Model::Loaded {
                db: _db,
                ctx,
                cache_strings: _,
                cache_towns: _,
                cache_counter,
            } => {
                println!(
                    "hit: {}, mis: {}, total: {}, hit fraction: {} ",
                    cache_counter.hit,
                    cache_counter.mis,
                    cache_counter.hit + cache_counter.mis,
                    f64::from(cache_counter.hit) / f64::from(cache_counter.hit + cache_counter.mis)
                );
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
                        entry.get().clone()
                    }
                    Entry::Vacant(entry) => {
                        cache_counter.mis += 1;
                        let value = Arc::new(db.get_towns_for_constraints(constraints)?);
                        entry.insert(value).clone()
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
                        entry.get().clone()
                    }
                    Entry::Vacant(entry) => {
                        cache_counter.mis += 1;
                        let value = Arc::new(db.get_names_for_constraint_type_in_constraints(
                            constraint_type,
                            constraints,
                        )?);
                        entry.insert(value).clone()
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
                        entry.get().clone()
                    }
                    Entry::Vacant(entry) => {
                        cache_counter.mis += 1;
                        let value = Arc::new(db.get_names_for_constraint_type(constraint_type)?);
                        entry.insert(value).clone()
                    }
                };
                Ok(value)
            }
        }
    }
}
