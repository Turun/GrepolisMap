use egui::TextBuffer;

use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt;
use std::sync::Arc;

use crate::constraint::Constraint;
use crate::town::Town;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum SelectionState {
    Loading,
    Finished,

    #[default]
    NewlyCreated,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TownSelection {
    #[serde(skip, default = "uuid::Uuid::new_v4")]
    uuid: uuid::Uuid,

    #[serde(default = "String::new")]
    pub name: String,

    #[serde(skip)]
    pub state: SelectionState,

    #[serde(default)]
    pub constraints: Vec<Constraint>,

    #[serde(default)]
    pub color: egui::Color32,

    #[serde(skip)]
    pub towns: Arc<Vec<Town>>,
}

impl TownSelection {
    /// Clone the `TownSelection`, but without the list of towns. Less memory
    /// required and we can reconstruct the list of towns anyway, if given
    /// the list of constraints.
    pub fn partial_clone(&self) -> Self {
        Self {
            towns: Arc::new(Vec::new()),
            uuid: self.uuid, // implements copy
            name: self.name.clone(),
            state: self.state, // implements copy
            constraints: self.constraints.clone(),
            color: self.color, // implements copy
        }
    }
}

impl Default for TownSelection {
    fn default() -> Self {
        let uuid = uuid::Uuid::new_v4();
        Self {
            uuid,
            name: uuid.to_string().char_range(0..6).to_owned(),
            state: SelectionState::NewlyCreated,
            towns: Arc::new(Vec::new()),
            constraints: vec![Constraint::default()],
            color: egui::Color32::GREEN,
        }
    }
}

impl PartialEq<TownSelection> for &mut TownSelection {
    fn eq(&self, other: &TownSelection) -> bool {
        self.uuid == other.uuid
    }
}

impl fmt::Display for TownSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TownSelection({} constraints, {} towns)",
            self.constraints.len(),
            self.towns.len()
        )
    }
}
