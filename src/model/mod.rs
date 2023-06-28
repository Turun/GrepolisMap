use std::time::Duration;

//the entry point for model
use crate::towns::{Constraint, ConstraintType, Town, TownSelection};

pub mod download;
mod offset_data;

pub enum Model {
    Uninitialized,
    Loaded {
        db: download::Database,
        ctx: egui::Context,
    },
}

// TODO: cache the methods here. This struct is replaced any time the Server/DB is changed, so we don't even have to think about cache invalidation

impl Model {
    pub fn request_repaint_after(&self, duration: Duration) {
        match self {
            Model::Uninitialized => { /*do nothing*/ }
            Model::Loaded { db: _db, ctx } => ctx.request_repaint_after(duration),
        }
    }

    pub fn get_towns_for_selection(&self, selection: &TownSelection) -> Vec<Town> {
        match self {
            Model::Uninitialized => return Vec::new(),
            Model::Loaded { db, ctx: _ctx } => db.get_towns_for_selection(selection),
        }
    }

    pub fn get_names_for_constraint_type_with_constraints(
        &self,
        constraint_type: &ConstraintType,
        constraints: &[&Constraint],
    ) -> Vec<String> {
        match self {
            Model::Uninitialized => return Vec::new(),
            Model::Loaded { db, ctx: _ctx } => {
                db.get_names_for_constraint_type_in_constraints(constraint_type, constraints)
            }
        }
    }

    pub fn get_ghost_towns(&self) -> Vec<Town> {
        match self {
            Model::Uninitialized => return Vec::new(),
            Model::Loaded { db, ctx: _ctx } => return db.get_ghost_towns(),
        }
    }

    pub fn get_all_towns(&self) -> Vec<Town> {
        match self {
            Model::Uninitialized => return Vec::new(),
            Model::Loaded { db, ctx: _ctx } => return db.get_all_towns(),
        }
    }

    pub fn get_names_for_constraint_type(&self, constraint_type: &ConstraintType) -> Vec<String> {
        match self {
            Model::Uninitialized => return Vec::new(),
            Model::Loaded { db, ctx: _ctx } => {
                return db.get_names_for_constraint_type(constraint_type)
            }
        }
    }
}
