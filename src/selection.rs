use core::fmt;
use std::collections::HashSet;
use std::default::Default;
use std::fmt::Display;
use std::sync::{mpsc, Arc};

use serde::{Deserialize, Serialize};

use crate::constraint::Constraint;
use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::message::MessageToModel;
use crate::town::Town;
use crate::view::{Change, Refresh};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
pub enum SelectionState {
    Loading,
    Finished,

    #[default]
    NewlyCreated,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
pub enum AndOr {
    #[default]
    And,
    Or,
}

impl Display for AndOr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AndOr::And => write!(f, "and"),
            AndOr::Or => write!(f, "or"),
        }
    }
}

impl AndOr {
    pub fn as_sql(self) -> String {
        match self {
            AndOr::And => String::from("AND"),
            AndOr::Or => String::from("OR"),
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(from = "EmptyTownSelection", into = "EmptyTownSelection")]
pub struct TownSelection {
    // TODO add a switch for and/or combinators
    // TODO add a toggle to collapse a selection in the GUI
    // TODO make it so that collapsed selections are no longer
    //requested from the Database and shown on the map. Or maybe
    //add an extra toggle for that. Or maybe make that bound to the
    //opacity of the chosen color
    pub collapsed: bool,
    pub name: String,
    pub state: SelectionState,
    pub constraints: Vec<Constraint>,
    pub constraint_join_mode: AndOr,
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

impl fmt::Display for TownSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TownSelection({}, {:?}, {:?}, {} towns)",
            self.name,
            self.state,
            self.constraints,
            self.towns.len()
        )
    }
}

impl TownSelection {
    /// If the color has an alpha value of 0, it is fully transparent and therefore invisible on the map
    pub fn is_hidden(&self) -> bool {
        self.color.a() == 0
    }

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
            constraint_join_mode: self.constraint_join_mode,
            color: self.color, // implements copy
        }
    }

    /// get a list of all `EmptyTownSelection`s that reference self, directly or indirectly. We need this so we can
    /// figure out which `TownSelections` need to be updated if self changes.
    pub fn get_dependents(
        &self,
        all_selections: &[EmptyTownSelection],
    ) -> HashSet<EmptyTownSelection> {
        let mut dependent_selections = HashSet::new();
        for selection in all_selections {
            // println!("checking {selection}");
            match selection.all_referenced_selections(all_selections) {
                Ok(list) => {
                    // let containts_bool = list.contains(&self.partial_clone());
                    let containts_bool = list
                        .iter()
                        .map(|ets| ets.name.as_str())
                        .any(|name| name == self.name);
                    if containts_bool {
                        dependent_selections.insert(selection.clone());
                    }
                }
                Err(err) => {
                    eprintln!("selection {selection} contains a cycle!: {err}");
                }
            }
        }
        // println!("got references to {dependent_selections:?}");
        dependent_selections
    }

    /// ask the backend to refresh this selection. Dependent selections must be refeshed independently
    pub fn refresh_self(
        &mut self,
        channel_tx: &mpsc::Sender<MessageToModel>,
        keep_ddv: HashSet<EmptyConstraint>,
        all_selections: &[EmptyTownSelection],
    ) {
        // Check if there is a cycle. If so, do not send to the backend
        // TODO inform the user of this!
        let referenced_selections = self
            .partial_clone()
            .all_referenced_selections(all_selections);
        if let Err(err) = referenced_selections {
            eprintln!("abort refresh: {err}");
            return;
        }

        // this check introduces a bug! If this check is commented in all
        // dependents of this selection spam the backend with update requests.
        // If this check is commented out, this does not happen. I dont know
        // why.
        // if !self.is_hidden() {
        self.state = SelectionState::Loading;
        for constraint in &mut self
            .constraints
            .iter_mut()
            .filter(|c| !keep_ddv.contains(&c.partial_clone()))
        {
            // drop the current list of values for all non-edited constraints
            // in other words, make sure it's refreshed but also dont flash the
            // list in the users face everytime they type a single character
            constraint.drop_down_values = None;
        }

        channel_tx
            .send(MessageToModel::FetchTowns(
                self.partial_clone(),
                keep_ddv,
                all_selections.to_vec(),
            ))
            .expect(&format!(
                "Failed to send Message to Model for Selection {}",
                self.partial_clone()
            ));
        // }
    }

    #[allow(clippy::too_many_lines)]
    pub fn make_ui(
        &mut self,
        ui: &mut egui::Ui,
        selection_index: usize,
    ) -> (Option<Change>, Refresh) {
        let mut re = None;
        let mut refresh_action = Refresh::None;
        let num_constraints = self.constraints.len();
        let mut edited_constraints = HashSet::new();
        let mut constraint_change_action = None;
        let mut constraint_join_mode_toggled = false;

        egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            ui.make_persistent_id(format!("collapsible header {selection_index}")),
            !self.collapsed,
        )
        .show_header(ui, |ui| {
            // add
            if ui.button("+").clicked() {
                re = Some(Change::Add);
            }
            // remove
            if ui.button("-").clicked() {
                re = Some(Change::Remove(selection_index));
            }
            // move up
            if ui.button("↑").clicked() {
                re = Some(Change::MoveUp(selection_index));
            }
            // move down
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
            // Color picker. Send a new request to the DB is the hidden status changed
            let previously_hidden = self.is_hidden();
            if ui.color_edit_button_srgba(&mut self.color).changed()
                && previously_hidden != self.is_hidden()
            {
                refresh_action = Refresh::InSitu(HashSet::new());
            }

            if self.is_hidden() {
                ui.label("Hidden");
            } else {
                ui.label(format!("{} Towns", self.towns.len()));
            }
            if self.state == SelectionState::Loading {
                ui.spinner();
            }
        })
        .body_unindented(|ui| {
            for (constraint_index, constraint) in self.constraints.iter_mut().enumerate() {
                let (change, edited, bool_toggled) = constraint.make_ui(
                    ui,
                    selection_index,
                    constraint_index,
                    constraint_index + 1 == num_constraints,
                    self.constraint_join_mode,
                );

                if bool_toggled {
                    constraint_join_mode_toggled = true;
                    self.constraint_join_mode = match self.constraint_join_mode {
                        AndOr::And => AndOr::Or,
                        AndOr::Or => AndOr::And,
                    }
                }

                if edited {
                    edited_constraints.insert(constraint.partial_clone());
                }

                if change.is_some() {
                    constraint_change_action = change;
                }
            }
        });

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
            (
                self.state,
                constraint_change_action,
                constraint_join_mode_toggled
            ),
            (SelectionState::NewlyCreated, _, _)  // reload everything if this selection is newly created (This is probably not needed, but I'll leave it in, just to be save)
                 | (_, Some(Change::Add | Change::Remove(_)), _) // or if a constraint was added or removed
            | (_, _, true) // or the join mode was switched (AND vs OR joining in SQL)
        );
        refresh_action = if refresh_complete_selection {
            Refresh::Complete
        } else if !edited_constraints.is_empty() {
            Refresh::InSitu(edited_constraints)
        } else {
            refresh_action
        };

        (re, refresh_action)
    }
}
