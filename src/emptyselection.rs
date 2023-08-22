use anyhow::Context;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use crate::emptyconstraint::EmptyConstraint;
use crate::selection::{SelectionState, TownSelection};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmptyTownSelection {
    #[serde(default = "String::new")]
    pub name: String,

    #[serde(default, with = "crate::emptyconstraint::short_serialization")]
    pub constraints: Vec<EmptyConstraint>,

    #[serde(default)]
    pub color: egui::Color32,
}

impl Default for EmptyTownSelection {
    fn default() -> Self {
        Self {
            name: Alphanumeric.sample_string(&mut rand::thread_rng(), 6), // https://stackoverflow.com/a/72977937
            constraints: vec![EmptyConstraint::default()],
            color: egui::Color32::GREEN,
        }
    }
}

impl PartialEq<EmptyTownSelection> for &mut EmptyTownSelection {
    fn eq(&self, other: &EmptyTownSelection) -> bool {
        self.name == other.name
            && self.constraints == other.constraints
            && self.color == other.color
    }
}
impl PartialEq<EmptyTownSelection> for EmptyTownSelection {
    fn eq(&self, other: &EmptyTownSelection) -> bool {
        self.name == other.name
            && self.constraints == other.constraints
            && self.color == other.color
    }
}

impl fmt::Display for EmptyTownSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EmptyTownSelection({} constraints)",
            self.constraints.len(),
        )
    }
}

impl EmptyTownSelection {
    pub fn fill(&self) -> TownSelection {
        TownSelection {
            name: self.name.clone(),
            state: SelectionState::default(),
            constraints: self.constraints.iter().map(EmptyConstraint::fill).collect(),
            color: self.color,
            towns: Arc::new(Vec::new()),
        }
    }

    pub fn directly_referenced_selection_names(&self) -> Vec<String> {
        self.constraints
            .iter()
            .filter_map(EmptyConstraint::referenced_selection)
            .collect()
    }

    pub fn directly_referenced_selections(&self, all_selections: &[Self]) -> Vec<Self> {
        let referenced_names = self.directly_referenced_selection_names();
        all_selections
            .iter()
            .cloned()
            .filter(|selection| referenced_names.contains(&selection.name))
            .collect()
    }

    /// Starting from self, create the tree of selection references.
    /// If a reference cycle is detected, return an error. If not,
    /// return the list of referenced `EmptyTownSelections`.
    pub fn all_referenced_selections(&self, all_selections: &[Self]) -> anyhow::Result<Vec<Self>> {
        let mut re = Vec::new();
        let mut referenced_names = self.directly_referenced_selection_names();
        let mut visited_names = vec![self.name.clone()];
        while let Some(name) = referenced_names.pop() {
            if let Some(selection) = all_selections
                .iter()
                .find(|selection| selection.name == name)
            {
                if visited_names.contains(&name) {
                    return Err(anyhow::format_err!(
                        "Circular Selection Reference Detected!"
                    ));
                }

                visited_names.push(name.clone());
                referenced_names.append(&mut selection.directly_referenced_selection_names());
                re.push(selection.clone());
            }
        }

        Ok(re)
    }

    pub fn try_from_str(text: &str) -> anyhow::Result<Vec<Self>> {
        // Attempt to parse text as a vector of selections, and it that doesn't work, parse it as a single selection.
        let res_parse_as_vec = serde_yaml::from_str(text);
        let res_parse_as_single = serde_yaml::from_str(text);

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
