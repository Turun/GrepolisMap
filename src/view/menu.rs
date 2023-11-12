use super::{
    preferences::{CacheSize, DarkModePref, Preferences},
    State, View,
};
use crate::{
    emptyselection::EmptyTownSelection,
    message::{MessageToModel, Progress},
    storage,
};
use arboard::Clipboard;
use native_dialog::FileDialog;
use std::collections::BTreeMap;

impl View {
    #[allow(clippy::too_many_lines)] // UI Code, am I right, hahah
    #[allow(clippy::single_match)] // temporary, until we fix the error reporting and make it more user friendly
    pub(crate) fn ui_menu(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // TODO [preferences] [auto delete saved data] after 1d/1w/1m/never

        egui::TopBottomPanel::top("menu bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                //////////////////////////////////////////////////////////////////////////////////
                ui.menu_button("Open Saved Data", |ui| {
                    let mut clicked_path = None;
                    for (server, saved_dbs) in &self.ui_data.saved_db {
                        ui.menu_button(server, |ui| {
                            for saved_db in saved_dbs {
                                if ui.button(format!("{saved_db}")).clicked() {
                                    clicked_path = Some(saved_db.clone());
                                    ui.close_menu();
                                }
                            }
                        });
                    }
                    if let Some(saved_db) = clicked_path {
                        self.reload_server();
                        self.channel_presenter_tx
                            .send(MessageToModel::LoadDataFromFile(saved_db.path, ctx.clone()))
                            .expect("Failed to send message to Model");
                        self.ui_state = State::Uninitialized(Progress::None);
                    }
                });

                //////////////////////////////////////////////////////////////////////////////////
                ui.menu_button("Delete Saved Data", |ui| {
                    ui.menu_button("Delete All", |ui| {
                        if ui.button("Yes, delete all saved data").clicked() {
                            storage::remove_all();
                            self.ui_data.saved_db = BTreeMap::new();
                            ui.close_menu();
                        }
                    });
                    let mut removed_dbs = Vec::new();
                    for (server, saved_dbs) in &self.ui_data.saved_db {
                        ui.menu_button(server, |ui| {
                            for saved_db in saved_dbs {
                                if ui.button(format!("{saved_db}")).clicked() {
                                    // TODO Error handling
                                    // TODO do it with messages instead?
                                    // TODO if we have a list of dbs in the backend, make sure this change is synchronized
                                    storage::remove_db(&saved_db.path).unwrap();
                                    removed_dbs.push(saved_db.clone());
                                }
                            }
                        });
                    }
                    for saved_dbs in &mut self.ui_data.saved_db.values_mut() {
                        saved_dbs.retain(|saved_db| !removed_dbs.contains(saved_db));
                    }
                });

                //////////////////////////////////////////////////////////////////////////////////
                ui.menu_button("Preferences", |ui| {
                    if ui.button("Darkmode").clicked() {
                        self.ui_data.apply_darkmode(ctx, DarkModePref::Dark);
                        ui.close_menu();
                    }
                    if ui
                        .button("Follow System Theme (Restart required)")
                        .clicked()
                    {
                        self.ui_data.apply_darkmode(ctx, DarkModePref::FollowSystem);
                        ui.close_menu();
                    }
                    if ui.button("Lightmode").clicked() {
                        self.ui_data.apply_darkmode(ctx, DarkModePref::Light);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("No Cache").clicked() {
                        self.ui_data.preferences.cache_size = CacheSize::None;
                        self.channel_presenter_tx
                            .send(MessageToModel::MaxCacheSize(CacheSize::None))
                            .expect("Failed to send MaxCacheSize message to backend");
                        ui.close_menu();
                    }
                    if ui.button("Normal Cache").clicked() {
                        self.ui_data.preferences.cache_size = CacheSize::Normal;
                        self.channel_presenter_tx
                            .send(MessageToModel::MaxCacheSize(CacheSize::Normal))
                            .expect("Failed to send MaxCacheSize message to backend");
                        ui.close_menu();
                    }
                    if ui.button("Large Cache").clicked() {
                        self.ui_data.preferences.cache_size = CacheSize::Large;
                        self.channel_presenter_tx
                            .send(MessageToModel::MaxCacheSize(CacheSize::Large))
                            .expect("Failed to send MaxCacheSize message to backend");
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button("Reset Preferences").clicked() {
                        self.ui_data.preferences = Preferences::default();
                        self.ui_data
                            .apply_darkmode(ctx, self.ui_data.preferences.darkmode);
                        Self::reset_saved_preferences(frame);
                        ui.close_menu();
                    }
                });

                //////////////////////////////////////////////////////////////////////////////////
                ui.menu_button("Import Selections", |ui| {
                    if ui.button("From Clipboard").clicked() {
                        match Clipboard::new() {
                            Ok(mut clipboard) => match clipboard.get_text() {
                                Ok(text) => {
                                    let result = EmptyTownSelection::try_from_str(&text);
                                    match result {
                                        Ok(town_selections) => {
                                            for town_selection in town_selections.iter().map(EmptyTownSelection::fill) {
                                                if !self.ui_data.selections.contains(&town_selection) {
                                                    self.ui_data.selections.push(town_selection);
                                                }
                                            }
                                        },
                                        Err(_) => {/* TODO report any errors to the user*/},
                                    }
                                }
                                Err(err) => {
                                    eprintln!("Got a Clipboard, but failed to get text from it: {err}");
                                }
                            },
                            Err(err) => {
                                eprintln!("Did not get the clipboard: {err}");
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("From File(s)").clicked() {
                        let files_res = FileDialog::new()
                            // .title("Choose one or more files to import selections")
                            .add_filter("Turun Map Selections", &["tms"])
                            .show_open_multiple_file();
                        match files_res {
                            Ok(files) => {
                                let results = EmptyTownSelection::try_from_path(&files);
                                for result in results{
                                    match result {
                                        Ok(town_selections) => {
                                            for town_selection in town_selections.iter().map(EmptyTownSelection::fill) {
                                                if !self.ui_data.selections.contains(&town_selection) {
                                                    self.ui_data.selections.push(town_selection);
                                                }
                                            }
                                        },
                                        Err(_) => {/* TODO report any errors to the user*/},
                                    }
                                }
                            }
                            Err(err) => {
                                eprintln!("Failed to open a file picker: {err}");
                            }
                        }
                    }
                });

                //////////////////////////////////////////////////////////////////////////////////
                ui.menu_button("Export Selections", |ui| {
                    if ui.button("To Clipboard").clicked() {
                        match Clipboard::new() {
                            Ok(mut clipboard) => {
                                let selections_yaml = serde_yaml::to_string(&self.ui_data.selections);
                                match selections_yaml {
                                    Ok(valid_yaml) => {
                                        if let Err(err) = clipboard.set_text(valid_yaml) {
                                            eprintln!("Failed to write YAML to clipboard: {err}");
                                        }
                                    }
                                    Err(err) => {
                                        eprintln!("Failed to convert the list of selections into Yaml: {err}");
                                    }
                                }
                            }
                            Err(err) => {
                                eprintln!("Did not get the clipboard: {err}");
                            }
                        }
                        ui.close_menu();
                    }
                    if ui.button("To File").clicked() {
                        let file_res = FileDialog::new()
                            .add_filter("Turun Map Selections", &["tms"])
                            .show_save_single_file();
                        match file_res {
                            Ok(file_opt) => {
                                if let Some(file_path) = file_opt {
                                    let selections_yaml = serde_yaml::to_string(&self.ui_data.selections);
                                    match selections_yaml {
                                        Ok(valid_yaml) => {
                                            if let Err(err) = std::fs::write(&file_path, valid_yaml) {
                                                eprintln!("Failed to write YAML to file ({file_path:?}) Error: {err:?}");
                                            }
                                        }
                                        Err(err) => {
                                            eprintln!("Failed to convert the list of selections into Yaml: {err:?}");
                                        }
                                    }
                                }else {
                                    /* ignore, the user knowingly clicked cancel*/
                                }
                            }
                            Err(err) => {
                                eprintln!("Failed to open a file chooser: {err:?}");
                            }
                        }
                    }
                });
            });
        });
    }
}
