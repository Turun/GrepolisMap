//the entry point for model

use crate::message::{FromType, Town, TownConstraint, TownSelection};

pub mod download;
mod offset_data;

pub enum Model {
    Uninitialized,
    Loaded { db: download::Database },
}

impl Model {
    pub fn get_towns_for_selection(&self, constraint: &TownConstraint) -> Vec<Town> {
        match self {
            Model::Uninitialized => return Vec::new(),
            Model::Loaded { db } => {
                // TODO
                match constraint.from_type {
                    FromType::Player => return db.get_towns_for_player(&constraint.value),
                    FromType::Alliance => return db.get_towns_for_alliance(&constraint.value),
                }
            }
        }
    }

    pub fn get_ghost_towns(&self) -> Vec<Town> {
        match self {
            Model::Uninitialized => return Vec::new(),
            Model::Loaded { db } => return db.get_ghost_towns(),
        }
    }
}
