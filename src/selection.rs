use anyhow::Context;
use egui::TextBuffer;
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt;
use std::path::PathBuf;
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

    pub fn try_from_str(text: &str) -> anyhow::Result<Vec<Self>> {
        // Attempt to parse text as a vector of selections, and it that doesn't work, parse it as a single selection.
        let res_parse_as_vec = serde_yaml::from_str(text);
        let res_parse_as_single = serde_yaml::from_str(text);

        // TODO for all new selections, check if they are already present. If so don't add them a second time.
        match (res_parse_as_vec, res_parse_as_single) {
            (Ok(vec), _) => Ok(vec),
            (Err(_err), Ok(selection)) => Ok(vec![selection]),
            (Err(err_vec), Err(err_single)) => {
                eprintln!("Could not parse text ({text}) as TownSelection (Error: {err_single:?}) or Vec<TownSelection> (Error: {err_vec:?}).");
                Err(
                    anyhow::Error::new(err_vec)
                    .context(err_single)
                    .context("Could not parse text ({text}) as TownSelection (Error: {single_err:?}) or Vec<TownSelection> (Error: {vec_err:?}).")
                )
            }
        }
    }

    pub fn try_from_path(files: &[PathBuf]) -> Vec<anyhow::Result<Vec<Self>>> {
        let mut re = Vec::with_capacity(files.len());
        for file in files {
            let content = std::fs::read_to_string(file);
            re.push(match content {
                Ok(content) => {
                    Self::try_from_str(&content).with_context(|| {format!("Failed to convert content of file {file:?} to an instance of TownSelection. Content of file is: {content}")})
                }
                Err(err) => {
                        Err(err)
                            .with_context(|| format!("Failed to read content of file {file:?}"))
                }
            });
        }
        re
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
