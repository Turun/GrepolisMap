use core::fmt;
use std::collections::HashSet;
use std::default::Default;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::constraint::Constraint;
use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::{EmptyTownSelection, HiddenId};
use crate::presenter::Presenter;
use crate::town::Town;
use crate::view::{Change, Refresh};

#[derive(
    Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default, PartialOrd, Ord,
)]
pub enum AndOr {
    #[default]
    And,
    Or,
}

impl ToString for AndOr {
    fn to_string(&self) -> String {
        match self {
            AndOr::And => t!("selection.and_or.and"),
            AndOr::Or => t!("selection.and_or.or"),
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
#[serde(from = "EmptyTownSelection", into = "EmptyTownSelection")] // TOOD: make this extra, so we can preserve collapsed state across app restarts
pub struct TownSelection {
    pub collapsed: bool,
    pub hidden_id: HiddenId,
    pub name: String,
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
            "TownSelection({}, {:?}, {} towns)",
            self.name,
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
            hidden_id: self.hidden_id.clone(),
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
        presenter: &mut Presenter,
        keep_ddv: &HashSet<EmptyConstraint>,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<()> {
        // Check if there is a cycle. If so, do not send to the backend
        // TODO inform the user of this!
        let referenced_selections = self
            .partial_clone()
            .all_referenced_selections(all_selections);
        if let Err(err) = referenced_selections {
            eprintln!("abort refresh: {err}");
            return Err(err);
        }

        // this check introduces a bug! If this check is commented in all
        // dependents of this selection spam the backend with update requests.
        // If this check is commented out, this does not happen. I dont know
        // why.
        // if !self.is_hidden() {
        for constraint in &mut self
            .constraints
            .iter_mut()
            .filter(|c| !keep_ddv.contains(&c.partial_clone()))
        {
            // drop the current list of values for all non-edited constraints
            // in other words, make sure it's refreshed but also dont flash the
            // list in the users face everytime they type a single character
            constraint.drop_down_values = None;
            // TODO: I think we can get rid of this drop mechanic. The list is refreshed whenever the constraint
            // gains focus. That should be enough in my opinion, be we need to make absolutely sure before we refactor
            // anything. If we do can can drop the keep_ddv arguement, which would cascade and remove a lot of code.
        }

        let fetch_towns_result =
            presenter.towns_for_selection(&self.partial_clone(), all_selections);
        return fetch_towns_result.map(|towns| self.towns = towns); // map Result<Arc<Vec<Towns>>> to Result<()> implicitly

        // }
    }

    #[allow(clippy::too_many_lines)]
    pub fn make_ui(
        &mut self,
        ui: &mut egui::Ui,
        presenter: &mut Presenter,
        all_selections: &[EmptyTownSelection],
        selection_index: usize,
    ) -> (Option<Change>, Refresh) {
        let mut re = None;
        let mut refresh_action = Refresh::None;
        let num_constraints = self.constraints.len();
        let mut edited_constraints = HashSet::new();
        let mut constraint_change_action = None;
        let mut constraint_join_mode_toggled = false;

        // TODO now it's not preserved across app restarts...
        //  because the TownSelection struct is using the EmptyTownSelection serde path.
        // TODO make TownSelection implement a serde path that is different from EmptyTownSelection.
        //  Or make EmptyTownSelection include the hidden_id, so that egui can persist the collapsed state
        let collapsing_header = egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            ui.make_persistent_id(format!("collapsible header {:?}", self.hidden_id)),
            !self.collapsed,
        );
        self.collapsed = !collapsing_header.is_open();
        collapsing_header
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
                    ui.label(t!("selection.hidden"));
                } else {
                    ui.label(t!("selection.town_count", count = self.towns.len()));
                }
            })
            .body(|ui| {
                let this_selection = self.partial_clone();
                for (constraint_index, constraint) in self.constraints.iter_mut().enumerate() {
                    let (change, edited, bool_toggled) = constraint.make_ui(
                        ui,
                        presenter,
                        &this_selection,
                        all_selections,
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
            (constraint_change_action, constraint_join_mode_toggled),
            (Some(Change::Add | Change::Remove(_)), _) // reload everything if a constraint was added or removed
            | (_, true) // or the join mode was switched
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
