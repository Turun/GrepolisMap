use super::{
    preferences::{CacheSize, DarkModePref, Language, Preferences, Telemetry},
    Progress, State, View,
};
use crate::emptyselection::EmptyTownSelection;
#[cfg(not(target_arch = "wasm32"))]
use crate::storage;
#[cfg(not(target_arch = "wasm32"))]
use arboard::Clipboard;
#[cfg(not(target_arch = "wasm32"))]
use native_dialog::FileDialog;
use rust_i18n::t;
use std::collections::BTreeMap;
use strum::IntoEnumIterator;

impl View {
    #[allow(clippy::too_many_lines)] // UI Code, am I right, hahah
    #[allow(clippy::single_match)] // temporary, until we fix the error reporting and make it more user friendly
    pub(crate) fn ui_menu(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // TODO [preferences] [auto delete saved data] after 1d/1w/1m/never
        // TODO add link to github
        // TODO make clipboard wasm capable

        egui::TopBottomPanel::top("menu bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                //////////////////////////////////////////////////////////////////////////////////
                #[cfg(not(target_arch="wasm32"))]
                ui.menu_button(t!("menu.open.title"), |ui| {
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
                        self.ui_data.server_id.clone_from(&saved_db.server_str);
                        // change self.ui_data
                        self.reload_server();
                        // tell the backend to fetch data from the server
                        // this cannot be done in the normal chunk of messages, it needs to be triggered before the normal round of messages
                        self.presenter.load_server_from_file(saved_db);
                        self.ui_state = State::Uninitialized(Progress::LoadingFile);
                    }
                });

                //////////////////////////////////////////////////////////////////////////////////
                #[cfg(not(target_arch="wasm32"))]
                ui.menu_button(t!("menu.delete.title"), |ui| {
                    ui.menu_button(t!("menu.delete.all"), |ui| {
                        if ui.button(t!("menu.delete.confirm")).clicked() {
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
                let text = t!("menu.preferences.title");
                ui.menu_button(text, |ui| {
                    if ui.button(t!("menu.preferences.darkmode")).clicked() {
                        self.ui_data.apply_darkmode(ctx, DarkModePref::Dark);
                        ui.close_menu();
                    }
                    if ui
                        .button(t!("menu.preferences.follow_system_theme"))
                        .clicked()
                    {
                        self.ui_data.apply_darkmode(ctx, DarkModePref::FollowSystem);
                        ui.close_menu();
                    }
                    if ui.button(t!("menu.preferences.lightmode")).clicked() {
                        self.ui_data.apply_darkmode(ctx, DarkModePref::Light);
                        ui.close_menu();
                    }

                    ui.separator();

                    if ui.button(t!("menu.preferences.no_cache")).clicked() {
                        self.ui_data.preferences.cache_size = CacheSize::None;
                        self.presenter.set_max_cache_size(CacheSize::None);
                        ui.close_menu();
                    }
                    if ui.button(t!("menu.preferences.normal_cache")).clicked() {
                        self.ui_data.preferences.cache_size = CacheSize::Normal;
                        self.presenter.set_max_cache_size(CacheSize::Normal);
                        ui.close_menu();
                    }
                    if ui.button(t!("menu.preferences.large_cache")).clicked() {
                        self.ui_data.preferences.cache_size = CacheSize::Large;
                        self.presenter.set_max_cache_size(CacheSize::Large);
                        ui.close_menu();
                    }

                    #[cfg(not(target_arch="wasm32"))]
                    {
                        ui.separator();

                        if ui.button(t!("menu.preferences.telemetry_all")).clicked() {
                            self.ui_data.preferences.telemetry = Telemetry::All;
                            ui.close_menu();
                        }
                        if ui.button(t!("menu.preferences.telemetry_version_check")).clicked() {
                            self.ui_data.preferences.telemetry = Telemetry::OnlyVersionCheck;
                            ui.close_menu();
                        }
                        if ui.button(t!("menu.preferences.telemetry_nothing")).clicked() {
                            self.ui_data.preferences.telemetry = Telemetry::Nothing;
                            ui.close_menu();
                        }
                    }

                    ui.separator();

                    for language in Language::iter() {
                        if ui.button(language.to_string()).clicked() {
                            language.apply();
                            self.ui_data.preferences.language = language;
                            ui.close_menu();
                        }
                    }

                    ui.separator();

                    if ui.button(t!("menu.preferences.reset")).clicked() {
                        self.ui_data.preferences = Preferences::default();
                        self.ui_data
                            .apply_darkmode(ctx, self.ui_data.preferences.darkmode);
                        Self::reset_saved_preferences(frame);
                        ui.close_menu();
                    }
                });

                //////////////////////////////////////////////////////////////////////////////////
                #[cfg(not(target_arch="wasm32"))]
                ui.menu_button(t!("menu.import.title"), |ui| {
                    if ui.button(t!("menu.import.from_clipboard")).clicked() {
                        match Clipboard::new() {
                            Ok(mut clipboard) => match clipboard.get_text() {
                                Ok(text) => {
                                    let result = EmptyTownSelection::try_from_str(&text);
                                    if let Ok(town_selections) = result {
                                        for town_selection in town_selections.iter().map(EmptyTownSelection::fill) {
                                            if !self.ui_data.selections.contains(&town_selection) {
                                                self.ui_data.selections.push(town_selection);
                                            }
                                        }
                                    } else {/* TODO report any errors to the user*/}
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
                    if ui.button(t!("menu.import.from_file")).clicked() {
                        let files_res = FileDialog::new()
                            // .title("Choose one or more files to import selections")
                            .add_filter("Turun Map Selections", &["tms"])
                            .show_open_multiple_file();
                        match files_res {
                            Ok(files) => {
                                let results = EmptyTownSelection::try_from_path(&files);
                                for result in results{
                                    if let Ok(town_selections) = result {
                                        for town_selection in town_selections.iter().map(EmptyTownSelection::fill) {
                                            if !self.ui_data.selections.contains(&town_selection) {
                                                self.ui_data.selections.push(town_selection);
                                            }
                                        }
                                    } else {/* TODO report any errors to the user*/}
                                }
                            }
                            Err(err) => {
                                eprintln!("Failed to open a file picker: {err}");
                            }
                        }
                    }
                });

                //////////////////////////////////////////////////////////////////////////////////
                #[cfg(not(target_arch="wasm32"))]
                ui.menu_button(t!("menu.export.title"), |ui| {
                    if ui.button(t!("menu.export.to_clipboard")).clicked() {
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
                    if ui.button(t!("menu.export.to_file")).clicked() {
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
