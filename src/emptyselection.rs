use anyhow::Context;
use rand::distributions::{Alphanumeric, DistString};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::default::Default;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::Arc;

use crate::emptyconstraint::EmptyConstraint;
use crate::selection::{AndOr, TownSelection};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HiddenId(String);

impl Default for HiddenId {
    fn default() -> Self {
        Self(Alphanumeric.sample_string(&mut rand::thread_rng(), 6))
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Serialize, Deserialize, Eq)]
pub struct EmptyTownSelection {
    #[serde(default = "String::new")]
    pub name: String,

    #[serde(skip)]
    pub hidden_id: HiddenId,

    #[serde(default, with = "crate::emptyconstraint::short_serialization")]
    pub constraints: Vec<EmptyConstraint>,

    #[serde(default)]
    pub constraint_join_mode: AndOr,

    #[serde(default)]
    pub color: egui::Color32,
}

impl Default for EmptyTownSelection {
    fn default() -> Self {
        Self {
            name: Alphanumeric.sample_string(&mut rand::thread_rng(), 6), // https://stackoverflow.com/a/72977937
            hidden_id: HiddenId::default(),
            constraints: vec![EmptyConstraint::default()],
            constraint_join_mode: AndOr::default(),
            color: egui::Color32::GREEN,
        }
    }
}

impl fmt::Display for EmptyTownSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EmptyTownSelection({}, {:?})",
            self.name, self.constraints
        )
    }
}

impl PartialEq for EmptyTownSelection {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.constraints == other.constraints
            && self.constraint_join_mode == other.constraint_join_mode
    }
}

impl Hash for EmptyTownSelection {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.constraints.hash(state);
        self.constraint_join_mode.hash(state);
    }
}

impl PartialOrd for EmptyTownSelection {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EmptyTownSelection {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let cmp_name = self.name.cmp(&other.name);
        if cmp_name != std::cmp::Ordering::Equal {
            return cmp_name;
        }

        let cmp_join = self.constraint_join_mode.cmp(&other.constraint_join_mode);
        if cmp_join != std::cmp::Ordering::Equal {
            return cmp_join;
        }

        for (e_self, e_other) in self.constraints.iter().zip(other.constraints.iter()) {
            let cmp_e = e_self.cmp(e_other);
            if cmp_e != std::cmp::Ordering::Equal {
                return cmp_e;
            }
        }

        return std::cmp::Ordering::Equal;
    }
}

impl EmptyTownSelection {
    /// If the color has an alpha value of 0, it is fully transparent and therefore invisible on the map
    pub fn is_hidden(&self) -> bool {
        self.color.a() == 0
    }

    pub fn fill(&self) -> TownSelection {
        TownSelection {
            collapsed: false, // The state of the headers is saved by egui by default. We don't do need to do that ourselves
            hidden_id: self.hidden_id.clone(),
            name: self.name.clone(),
            constraints: self.constraints.iter().map(EmptyConstraint::fill).collect(),
            constraint_join_mode: self.constraint_join_mode,
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
            .filter(|&selection| referenced_names.contains(&selection.name))
            .cloned()
            .collect()
    }

    /// Starting from self, create the tree of selection references.
    /// If a reference cycle is detected, return an error. If not,
    /// return the list of referenced `EmptyTownSelections`.
    pub fn all_referenced_selections(
        &self,
        all_selections: &[Self],
    ) -> anyhow::Result<BTreeSet<Self>> {
        if self.contains_circular_reference(all_selections) {
            return Err(anyhow::format_err!(
                "Circular Selection Reference Detected!"
            ));
        }

        let mut re = BTreeSet::new();
        let mut referenced_names = self.directly_referenced_selection_names();
        while let Some(name) = referenced_names.pop() {
            if let Some(selection) = all_selections
                .iter()
                .find(|selection| selection.name == name)
            {
                referenced_names.append(&mut selection.directly_referenced_selection_names());
                re.insert(selection.clone());
            }
        }

        Ok(re)
    }

    /// performs depth first search to detect if there are any cycles in the constraints. A cycle
    /// can happen if the user uses the `InSelction` or `NotInSelection` constraint comparators,
    /// which links to another selection, which in turn can link to more selections
    pub fn contains_circular_reference(&self, all_selections: &[Self]) -> bool {
        // from https://stackoverflow.com/a/53995651/14053391
        let mut discovered: HashSet<Self> = HashSet::new();
        let mut finished: HashSet<Self> = HashSet::new();
        Self::depth_first_search(all_selections, self, &mut finished, &mut discovered)
    }

    fn depth_first_search(
        all_selections: &[Self],
        leaf_selection: &Self,
        finished: &mut HashSet<Self>,
        discovered: &mut HashSet<Self>,
    ) -> bool {
        let mut cycle_detected = false;
        discovered.insert(leaf_selection.clone());

        for referenced_selection in leaf_selection.directly_referenced_selections(all_selections) {
            if discovered.contains(&referenced_selection) {
                eprintln!("Cycle detected: found a back edge from {leaf_selection} to {referenced_selection}.");
                cycle_detected = true;
                break;
            }

            // short curcuits, so we don't have to worry about side effect
            if !finished.contains(&referenced_selection)
                && Self::depth_first_search(
                    all_selections,
                    &referenced_selection,
                    finished,
                    discovered,
                )
            {
                cycle_detected = true;
            }
        }

        let _was_present = discovered.remove(leaf_selection);
        finished.insert(leaf_selection.clone());

        cycle_detected
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
