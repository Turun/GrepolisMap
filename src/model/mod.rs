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

impl Model {
    pub fn request_repaint(&self) {
        match self {
            Model::Uninitialized => { /*do nothing*/ }
            Model::Loaded { db: _db, ctx } => ctx.request_repaint(),
        }
    }

    pub fn get_towns_for_selection(&self, selection: &TownSelection) -> Vec<Town> {
        match self {
            Model::Uninitialized => return Vec::new(),
            Model::Loaded { db, ctx: _ctx } => db.get_towns_for_selection(selection),
        }
    }

    pub fn get_towns_for_constraint_with_selection(
        &self,
        constraint: &Constraint,
        selection: &TownSelection,
    ) -> Vec<String> {
        let mut selection = selection.partial_clone();
        let index_opt = selection.constraints.iter().position(|c| c == constraint);
        if let Some(index) = index_opt {
            selection.constraints.swap_remove(index);
            match self {
                Model::Uninitialized => return Vec::new(),
                Model::Loaded { db, ctx: _ctx } => db.get_names_for_constraint_type_in_selection(
                    &constraint.constraint_type,
                    &selection,
                ),
            }
        } else {
            return Vec::new();
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
