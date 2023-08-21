use std::collections::HashSet;
use std::default::Default;
use std::sync::{mpsc, Arc};

use serde::{Deserialize, Serialize};

use crate::constraint::Constraint;
use crate::emptyselection::EmptyTownSelection;
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
#[serde(from = "EmptyTownSelection", into = "EmptyTownSelection")]
pub struct TownSelection {
    pub name: String,
    pub state: SelectionState,
    pub constraints: Vec<Constraint>,
    pub color: egui::Color32,
    pub towns: Arc<Vec<Town>>,
}

// required for serde
impl From<TownSelection> for EmptyTownSelection {
    fn from(val: TownSelection) -> Self {
        val.partial_clone()
    }
}

// required for serde
impl From<EmptyTownSelection> for TownSelection {
    fn from(other: EmptyTownSelection) -> Self {
        other.fill()
    }
}

impl Default for TownSelection {
    fn default() -> Self {
        EmptyTownSelection::default().fill()
    }
}

impl PartialEq for TownSelection {
    fn eq(&self, other: &Self) -> bool {
        self.partial_clone() == other.partial_clone()
    }
}
impl PartialEq<EmptyTownSelection> for TownSelection {
    fn eq(&self, other: &EmptyTownSelection) -> bool {
        &self.partial_clone() == other
    }
}
impl PartialEq<EmptyTownSelection> for &mut TownSelection {
    fn eq(&self, other: &EmptyTownSelection) -> bool {
        &self.partial_clone() == other
    }
}

impl TownSelection {
    /// Clone the `TownSelection`, but without the list of towns. Less memory
    /// required and we can reconstruct the list of towns anyway, if given
    /// the list of constraints.
    pub fn partial_clone(&self) -> EmptyTownSelection {
        EmptyTownSelection {
            name: self.name.clone(),
            constraints: self
                .constraints
                .iter()
                .map(Constraint::partial_clone)
                .collect(),
            color: self.color, // implements copy
        }
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
                self.partial_clone()
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

            if change.is_some() {
                constraint_change_action = change;
            }
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
                .filter(|c| !edited_constraints.contains(&c.partial_clone()))
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
                    &self.partial_clone()
                ));
        }

        re
    }
}
