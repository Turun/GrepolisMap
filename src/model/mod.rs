use std::{sync::Arc, time::Duration};

use eframe::epaint::ahash::HashMap;

//the entry point for model
use crate::towns::{Constraint, ConstraintType, Town};

pub mod download;
mod offset_data;

pub enum Model {
    Uninitialized,
    Loaded {
        db: download::Database,
        ctx: egui::Context,
        cache_strings: HashMap<(ConstraintType, Vec<Constraint>), Arc<Vec<String>>>,
        cache_towns: HashMap<Vec<Constraint>, Arc<Vec<Town>>>,
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
            } => ctx.request_repaint_after(duration),
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
            } => Ok(cache_towns
                .entry(constraints.to_vec())
                .or_insert(Arc::new(db.get_towns_for_constraints(constraints)?))
                .clone()),
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
            } => Ok(cache_strings
                .entry((constraint_type, constraints.to_vec()))
                .or_insert(Arc::new(db.get_names_for_constraint_type_in_constraints(
                    constraint_type,
                    constraints,
                )?))
                .clone()),
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
            } => Ok(cache_strings
                .entry((constraint_type, Vec::new()))
                .or_insert(Arc::new(db.get_names_for_constraint_type(constraint_type)?))
                .clone()),
        }
    }
}
