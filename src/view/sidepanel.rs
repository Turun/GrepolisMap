use std::collections::HashSet;
use std::sync::Arc;

use super::Change;
use super::Progress;
use super::State;
use super::View;
use crate::emptyselection::EmptyTownSelection;
use crate::selection::TownSelection;

impl View {
    #[allow(clippy::too_many_lines)]
    pub fn ui_sidepanel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                self.ui_server_input(ui);
                ui.label(t!(
                    "sidepanel.town_stats.total",
                    count = self.ui_data.all_towns.len()
                ));
                ui.label(t!(
                    "sidepanel.town_stats.ghosts",
                    count = self.ui_data.ghost_towns.len()
                ));
                ui.separator();

                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.ui_data.settings_all.enabled, "");
                    ui.label(t!("sidepanel.town_toggle.all"));
                    ui.color_edit_button_srgba(&mut self.ui_data.settings_all.color);
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.ui_data.settings_ghosts.enabled, "");
                    ui.label(t!("sidepanel.town_toggle.ghosts"));
                    ui.color_edit_button_srgba(&mut self.ui_data.settings_ghosts.color);
                });
                ui.separator();

                let all_selections: Vec<EmptyTownSelection> = self
                    .ui_data
                    .selections
                    .iter()
                    .map(TownSelection::partial_clone)
                    .collect();

                egui::ScrollArea::vertical().show(ui, |ui| {
                    let mut selection_change_action: Option<Change> = None;
                    let mut refresh_list = Vec::new();
                    for (selection_index, selection) in
                        self.ui_data.selections.iter_mut().enumerate()
                    {
                        let (opt_change, refresh) = selection.make_ui(
                            ui,
                            &mut self.presenter,
                            &all_selections,
                            selection_index,
                        );
                        if let Some(change) = opt_change {
                            selection_change_action = Some(change);
                        }
                        refresh_list.push((selection_index, refresh));
                        ui.separator();
                    }

                    // process selections which need a refresh
                    // let mut all_dependent_selections = HashSet::new();
                    for (index, refresh) in refresh_list {
                        let selection = self.ui_data.selections.get_mut(index).unwrap();
                        let edited_constraints = match refresh {
                            super::Refresh::Complete => {
                                // println!("refresh complete selection for {selection}");
                                selection.towns = Arc::new(Vec::new());
                                HashSet::new()
                            }
                            super::Refresh::InSitu(edited_constraints) => {
                                // println!("refresh edited constraints for {selection}");
                                // TODO: also drop selection.towns here as well. Not dropping it is a bug!
                                edited_constraints
                            }
                            super::Refresh::None => {
                                continue;
                            }
                        };

                        // refresh the selection that is currently being edited with the caveat that some constraints are currently being edited and we should maybe not change their drop down values
                        let result = selection.refresh_self(
                            &mut self.presenter,
                            &edited_constraints,
                            &all_selections,
                        );
                        if let Err(err) = result {
                            self.ui_state =
                                State::Uninitialized(Progress::BackendCrashed(format!("{err:?}")));
                        }
                        // and refresh all selections that depend on the currently edited one completely
                        let dependents = selection.get_dependents(&all_selections);
                        // all_dependent_selections.extend(dependents);
                        for dependent_selection in dependents {
                            let selection = self
                                .ui_data
                                .selections
                                .iter_mut()
                                .find(|mutable_selection| {
                                    mutable_selection.name == dependent_selection.name
                                })
                                .expect("This Should not happen");
                            let result = selection.refresh_self(
                                &mut self.presenter,
                                &HashSet::new(),
                                &all_selections,
                            );
                            if let Err(err) = result {
                                self.ui_state = State::Uninitialized(Progress::BackendCrashed(
                                    format!("{err:?}"),
                                ));
                            }
                        }
                    }

                    // process changes regarding the number or order of selection uis
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
        });
    }
}
