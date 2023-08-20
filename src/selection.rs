use anyhow::Context;
use egui::TextBuffer;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::default::Default;
use std::fmt;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};

use crate::constraint::Constraint;
use crate::message::MessageToModel;
use crate::town::Town;
use crate::view::Change;

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

    pub fn refresh(&mut self, channel_tx: &mpsc::Sender<MessageToModel>) {
        self.state = SelectionState::Loading;
        for constraint in &mut self.constraints {
            constraint.drop_down_values = None;
        }

        channel_tx
            .send(MessageToModel::FetchTowns(
                self.partial_clone(),
                HashSet::new(),
            ))
            .expect(&format!(
                "Failed to send Message to Model for Selection {}",
                &self
            ));
    }

    pub fn make_ui(
        &mut self,
        ui: &mut egui::Ui,
        channel_tx: &mpsc::Sender<MessageToModel>,
        selection_index: usize,
    ) -> Option<Change> {
        let mut re = None;

        let _first_row_response = ui.horizontal(|ui| {
            // TODO make the selection collapsible
            if ui.button("+").clicked() {
                re = Some(Change::Add);
            }
            if ui.button("-").clicked() {
                re = Some(Change::Remove(selection_index));
            }
            if ui.button("↑").clicked() {
                re = Some(Change::MoveUp(selection_index));
            }
            if ui.button("↓").clicked() {
                re = Some(Change::MoveDown(selection_index));
            }
            ui.add_sized(
                [
                    ui.style().spacing.interact_size.x * 6.0,
                    ui.style().spacing.interact_size.y,
                ],
                egui::TextEdit::singleline(&mut self.name),
            );
            ui.color_edit_button_srgba(&mut self.color);
            ui.label(format!("{} Towns", self.towns.len()));
            if self.state == SelectionState::Loading {
                ui.spinner();
            }
        });

        let num_constraints = self.constraints.len();
        let mut edited_constraints = HashSet::new();
        let mut constraint_change_action = None;
        for (constraint_index, constraint) in self.constraints.iter_mut().enumerate() {
            let (change, edited) = constraint.make_ui(
                ui,
                selection_index,
                constraint_index,
                constraint_index + 1 == num_constraints,
            );

            if edited {
                edited_constraints.insert(constraint.partial_clone());
            }

            constraint_change_action = change;
        }
        if let Some(change) = constraint_change_action {
            match change {
                Change::MoveUp(index) => {
                    if index >= 1 {
                        self.constraints.swap(index, index - 1);
                    }
                }
                Change::Remove(index) => {
                    let _element = self.constraints.remove(index);
                    if self.constraints.is_empty() {
                        // ensure there is always at least one constraint
                        self.constraints.push(Constraint::default());
                    }
                }
                Change::MoveDown(index) => {
                    if index + 1 < self.constraints.len() {
                        self.constraints.swap(index, index + 1);
                    }
                }
                Change::Add => self.constraints.push(Constraint::default()),
            }
        }

        let refresh_complete_selection = matches!(
            (self.state, constraint_change_action),
            (SelectionState::NewlyCreated, _)  // reload everything if this selection is newly created (This is probably not needed, but I'll leave it in, just to be save)
                 | (_, Some(Change::Add | Change::Remove(_))) // or if a constraint was added or removed
        );
        if refresh_complete_selection {
            self.towns = Arc::new(Vec::new());
            self.refresh(channel_tx);
        } else if !edited_constraints.is_empty() {
            self.state = SelectionState::Loading;
            for constraint in &mut self
                .constraints
                .iter_mut()
                .filter(|c| !edited_constraints.contains(c))
            {
                // the ddvs of all constraints that were not edited are invalidated.
                constraint.drop_down_values = None;
            }

            channel_tx
                .send(MessageToModel::FetchTowns(
                    self.partial_clone(),
                    edited_constraints,
                ))
                .expect(&format!(
                    "Failed to send Message to Model for selection {}",
                    &self
                ));
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
