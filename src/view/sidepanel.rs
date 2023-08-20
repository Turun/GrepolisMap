use crate::constraint::Constraint;
use crate::message::MessageToModel;
use crate::selection::SelectionState;
use crate::selection::TownSelection;
use std::collections::HashSet;
use std::sync::Arc;

use super::Change;
use super::View;

impl View {
    #[allow(clippy::too_many_lines)] // UI Code, am I right, hahah
    pub fn ui_sidepanel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                self.ui_server_input(ui, ctx);
                ui.label(format!("Total Towns: {}", self.ui_data.all_towns.len()));
                ui.label(format!("Ghost Towns: {}", self.ui_data.ghost_towns.len()));
                ui.separator();

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.ui_data.settings_all.enabled, "");
                    ui.label("All Towns:");
                    ui.color_edit_button_srgba(&mut self.ui_data.settings_all.color);
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.ui_data.settings_ghosts.enabled, "");
                    ui.label("Ghost Towns:");
                    ui.color_edit_button_srgba(&mut self.ui_data.settings_ghosts.color);
                });
                ui.separator();

                let mut selection_change_action: Option<Change> = None;
                for (selection_index, selection) in self.ui_data.selections.iter_mut().enumerate() {
                    let _first_row_response = ui.horizontal(|ui| {
                        if ui.button("+").clicked() {
                            selection_change_action = Some(Change::Add);
                        }
                        if ui.button("-").clicked() {
                            selection_change_action = Some(Change::Remove(selection_index));
                        }
                        if ui.button("↑").clicked() {
                            selection_change_action = Some(Change::MoveUp(selection_index));
                        }
                        if ui.button("↓").clicked() {
                            selection_change_action = Some(Change::MoveDown(selection_index));
                        }
                        ui.add_sized(
                            [
                                ui.style().spacing.interact_size.x * 6.0,
                                ui.style().spacing.interact_size.y,
                            ],
                            egui::TextEdit::singleline(&mut selection.name),
                        );
                        ui.color_edit_button_srgba(&mut selection.color);
                        ui.label(format!("{} Towns", selection.towns.len()));
                        if selection.state == SelectionState::Loading {
                            ui.spinner();
                        }
                    });

                    let num_constraints = selection.constraints.len();
                    let mut edited_constraints = HashSet::new();
                    let mut constraint_change_action = None;
                    for (constraint_index, constraint) in
                        selection.constraints.iter_mut().enumerate()
                    {
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
                                    selection.constraints.swap(index, index - 1);
                                }
                            }
                            Change::Remove(index) => {
                                let _element = selection.constraints.remove(index);
                                if selection.constraints.is_empty() {
                                    // ensure there is always at least one constraint
                                    selection.constraints.push(Constraint::default());
                                }
                            }
                            Change::MoveDown(index) => {
                                if index + 1 < selection.constraints.len() {
                                    selection.constraints.swap(index, index + 1);
                                }
                            }
                            Change::Add => selection.constraints.push(Constraint::default()),
                        }
                    }

                    let refresh_complete_selection = matches!(
                        (selection.state, constraint_change_action),
                        (SelectionState::NewlyCreated, _)  // reload everything if this selection is newly created (This is probably not needed, but I'll leave it in, just to be save)
                            | (_, Some(Change::Add | Change::Remove(_))) // or if a constraint was added or removed
                    );
                    if refresh_complete_selection {
                        selection.towns = Arc::new(Vec::new());
                        selection.refresh(&self.channel_presenter_tx);
                    } else if !edited_constraints.is_empty() {
                        selection.state = SelectionState::Loading;
                        for constraint in &mut selection
                            .constraints
                            .iter_mut()
                            .filter(|c| !edited_constraints.contains(c))
                        {
                            // the ddvs of all constraints that were not edited are invalidated.
                            constraint.drop_down_values = None;
                        }

                        self.channel_presenter_tx
                            .send(MessageToModel::FetchTowns(
                                selection.partial_clone(),
                                edited_constraints,
                            ))
                            .expect(&format!(
                                "Failed to send Message to Model for Selection {}",
                                &selection
                            ));
                    }
                    ui.separator();
                }

                if let Some(change_action) = selection_change_action {
                    match change_action {
                        Change::MoveUp(index) => {
                            if index >= 1 {
                                self.ui_data.selections.swap(index, index - 1);
                            }
                        }
                        Change::Remove(index) => {
                            let _elem = self.ui_data.selections.remove(index);
                            if self.ui_data.selections.is_empty() {
                                // ensure there is always at least one selection
                                self.ui_data.selections.push(TownSelection::default());
                            }
                        }
                        Change::MoveDown(index) => {
                            if index + 1 < self.ui_data.selections.len() {
                                self.ui_data.selections.swap(index, index + 1);
                            }
                        }
                        Change::Add => {
                            self.ui_data.selections.push(TownSelection::default());
                        }
                    }
                }
            });
        });
    }
}
