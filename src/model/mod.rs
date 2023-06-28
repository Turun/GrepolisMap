//the entry point for model

use crate::message::{Town, TownSelection};

pub mod download;
mod offset_data;

pub enum Model {
    Uninitialized,
    Loaded { db: download::Database },
}

impl Model {
    pub fn get_towns_for_selection(&self, selection: &TownSelection) -> Vec<Town> {
        match self {
            Model::Uninitialized => return Vec::new(),
            Model::Loaded { db } => match selection {
                TownSelection::None => return Vec::new(),
                TownSelection::All => return db.get_all_towns(),
                TownSelection::Ghosts => return db.get_ghost_towns(),
                TownSelection::Selected(_) => todo!(),
            },
        }
    }
}
