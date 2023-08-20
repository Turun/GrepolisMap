use crate::constraint::Comparator;
use crate::constraint::Constraint;
use crate::constraint::ConstraintType;
use crate::message::MessageToModel;
use crate::selection::SelectionState;
use crate::selection::TownSelection;
use crate::view::dropdownbox::DropDownBox;
use std::collections::HashSet;
use strum::IntoEnumIterator;

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
                for (index, selection) in self.ui_data.selections.iter_mut().enumerate() {
                    let _first_row_response = ui.horizontal(|ui| {
                        if ui.button("+").clicked() {
                            selection_change_action = Some(Change::Add);
                        }
                        if ui.button("-").clicked() {
                            selection_change_action = Some(Change::Remove(index));
                        }
                        if ui.button("↑").clicked() {
                            selection_change_action = Some(Change::MoveUp(index));
                        }
                        if ui.button("↓").clicked() {
                            selection_change_action = Some(Change::MoveDown(index));
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
                    let mut refresh_complete_selection =
                        selection.state == SelectionState::NewlyCreated;
                    let mut edited_constraints = HashSet::new();
                    let mut constraint_change_action = None;
                    for (cindex, constraint) in selection.constraints.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            let _inner_response = egui::ComboBox::from_id_source(format!(
                                "ComboxBox {index}/{cindex} Type"
                            ))
                            .width(ui.style().spacing.interact_size.x * 3.5)
                            .selected_text(format!("{}", constraint.constraint_type))
                            .show_ui(ui, |ui| {
                                for value in ConstraintType::iter() {
                                    let text = value.to_string();
                                    if ui
                                        .selectable_value(
                                            &mut constraint.constraint_type,
                                            value,
                                            text,
                                        )
                                        .clicked()
                                    {
                                        edited_constraints.insert(constraint.partial_clone());
                                    }
                                }
                            });

                            let _inner_response = egui::ComboBox::from_id_source(format!(
                                "ComboxBox {index}/{cindex} Comparator"
                            ))
                            .width(ui.style().spacing.interact_size.x * 1.75)
                            .selected_text(format!("{}", constraint.comparator))
                            .show_ui(ui, |ui| {
                                for value in Comparator::iter() {
                                    let text = value.to_string();
                                    if ui
                                        .selectable_value(&mut constraint.comparator, value, text)
                                        .clicked()
                                    {
                                        edited_constraints.insert(constraint.partial_clone());
                                    }
                                }
                            });

                            let ddb = DropDownBox::from_iter(
                                constraint.drop_down_values.as_ref(),
                                format!("ComboBox {index}/{cindex} Value"),
                                &mut constraint.value,
                            );
                            if ui
                                .add_sized(
                                    [
                                        ui.style().spacing.interact_size.x * 4.5,
                                        ui.style().spacing.interact_size.y,
                                    ],
                                    ddb,
                                )
                                .changed()
                            {
                                edited_constraints.insert(constraint.partial_clone());
                            };
                            if cindex + 1 == num_constraints {
                                if ui.button("+").clicked() {
                                    constraint_change_action = Some(Change::Add);
                                    refresh_complete_selection = true;
                                }
                            } else {
                                ui.label("and");
                            }
                            if ui.button("-").clicked() {
                                constraint_change_action = Some(Change::Remove(cindex));
                                refresh_complete_selection = true;
                            }
                            if ui.button("↑").clicked() {
                                constraint_change_action = Some(Change::MoveUp(cindex));
                            }
                            if ui.button("↓").clicked() {
                                constraint_change_action = Some(Change::MoveDown(cindex));
                            }
                        });
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

                    if refresh_complete_selection {
                        selection.state = SelectionState::Loading;
                        for constraint in &mut selection.constraints {
                            constraint.drop_down_values = None;
                        }

                        self.channel_presenter_tx
                            .send(MessageToModel::FetchTowns(
                                selection.partial_clone(),
                                HashSet::new(),
                            ))
                            .expect(&format!(
                                "Failed to send Message to Model for Selection {}",
                                &selection
                            ));
                    } else if !edited_constraints.is_empty() {
                        selection.state = SelectionState::Loading;
                        for constraint in &mut selection
                            .constraints
                            .iter_mut()
                            .filter(|c| !edited_constraints.contains(c))
                        {
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
