mod data;
pub(crate) mod dropdownbox;
mod map;
mod menu;
pub(crate) mod preferences;
mod selectable_label;
mod sidepanel;

use crate::message::{MessageToModel, MessageToView, Progress, Server};
use crate::selection::SelectionState;
use crate::view::data::Data;
use crate::VERSION;
use eframe::Storage;
use egui::{FontData, ProgressBar, RichText, Ui};
use std::sync::{mpsc, Arc};
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum Change {
    Add,
    MoveUp(usize),
    Remove(usize),
    MoveDown(usize),
}

#[derive(Debug, Clone)]
pub enum State {
    Uninitialized(Progress),
    Show,
}

pub struct View {
    ui_state: State,
    ui_data: Data,
    channel_presenter_rx: mpsc::Receiver<MessageToView>,
    channel_presenter_tx: mpsc::Sender<MessageToModel>,
}

impl View {
    fn setup(
        cc: &eframe::CreationContext,
        rx: mpsc::Receiver<MessageToView>,
        tx: mpsc::Sender<MessageToModel>,
    ) -> Self {
        let mut re = Self {
            ui_state: State::Uninitialized(Progress::None),
            ui_data: Data::default(),
            channel_presenter_rx: rx,
            channel_presenter_tx: tx,
        };

        // include a Unicode font and make it the default
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            String::from("Custom Font"),
            FontData::from_static(include_bytes!("../../NotoSansJP-Regular.ttf")),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push(String::from("Custom Font"));
        cc.egui_ctx.set_fonts(fonts);

        re.channel_presenter_tx
            .send(MessageToModel::DiscoverSavedDatabases)
            .expect("Failed to send message to backend: Discover Saved Databases");

        // load saved app data from disk
        if let Some(storage) = cc.storage {
            re.ui_data = if let Some(text) = storage.get_string(eframe::APP_KEY) {
                serde_yaml::from_str(&text).unwrap_or_else(|err| {
                    eprintln!("Failed to read saved config as YAML: {err}");
                    Data::default()
                })
            } else {
                println!("No previously saved preferences found");
                Data::default()
            };
        }

        re.channel_presenter_tx
            .send(MessageToModel::MaxCacheSize(
                re.ui_data.preferences.cache_size,
            ))
            .expect("Failed to send message to backend: MaxCacheSize");

        // TODO
        // self.channel_presenter_tx
        //     .send(MessageToModel::AutoDeleteTime(data.preferences.auto_delete_time))
        //     .expect("Failed to send message to backend: Discover Saved Databases");

        re.ui_data
            .apply_darkmode(&cc.egui_ctx, re.ui_data.preferences.darkmode);

        re
    }

    pub fn new_and_start(rx: mpsc::Receiver<MessageToView>, tx: mpsc::Sender<MessageToModel>) {
        let native_options = eframe::NativeOptions {
            // defaults to window title, but we include the version in the window title. Since
            // it should stay the same across version changes we give it a fixed value here.
            app_id: Some("Turun Map".to_owned()),
            ..eframe::NativeOptions::default()
        };
        eframe::run_native(
            &format!("Turun Map {VERSION}"),
            native_options,
            Box::new(|cc| Box::new(View::setup(cc, rx, tx))),
        )
        .expect("Eframe failed!");
    }

    fn reset_saved_preferences(frame: &mut eframe::Frame) {
        if let Some(storage) = frame.storage_mut() {
            storage.set_string(eframe::APP_KEY, String::new());
            storage.flush();
        }
    }

    /// reloading a server mean we should partially copy our `ui_data` and reset the data associated with selections
    fn reload_server(&mut self) {
        self.ui_state = State::Uninitialized(Progress::None);
        // TODO: do not keep the self.ui_data.canvas position the same when we switch servers. But only then!
        self.ui_data = Data {
            all_towns: Arc::new(Vec::new()),
            ghost_towns: Arc::new(Vec::new()),
            ..self.ui_data.clone()
        };
        // the selections are invalidated after the backend sends "got server"
    }

    fn ui_server_input(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let mut should_load_server = false;
        ui.horizontal(|ui| {
            ui.label("Server ID");
            let response = ui.text_edit_singleline(&mut self.ui_data.server_id);
            if response.lost_focus()
                && response
                    .ctx
                    .input(|input| input.key_pressed(egui::Key::Enter))
            {
                // detect enter on text field: https://github.com/emilk/egui/issues/229
                should_load_server = true;
            }
        });
        if ui
            .add(egui::Button::new("Load Data for this Server"))
            .clicked()
        {
            should_load_server = true;
        }

        if should_load_server {
            // change self.ui_data
            self.reload_server();
            // tell the backend to fetch data from the server
            self.channel_presenter_tx
                .send(MessageToModel::SetServer(
                    Server {
                        id: self.ui_data.server_id.clone(),
                    },
                    ctx.clone(),
                ))
                .expect("Failed to send the SetServer Message to the backend");
            // refresh our list of available saved databases
            self.channel_presenter_tx
                .send(MessageToModel::DiscoverSavedDatabases)
                .expect("Failed to send Discover Saved Databases to server");
        }
    }

    fn ui_uninitialized(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        progress: Progress,
    ) {
        self.ui_menu(ctx, frame);
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                self.ui_server_input(ui, ctx);
                match progress {
                    Progress::None => {}
                    Progress::BackendCrashed => {
                        ui.label(
                            RichText::new("The Database Crashed. Please Reload The Data.")
                                .color(ui.style().visuals.warn_fg_color),
                        );
                    }
                    Progress::Started => {
                        ui.add(ProgressBar::new(0.0).text(format!("{progress:?}")));
                    }
                    Progress::IslandOffsets => {
                        ui.add(ProgressBar::new(0.2).text(format!("{progress:?}")));
                    }
                    Progress::Alliances => {
                        ui.add(ProgressBar::new(0.4).text(format!("{progress:?}")));
                    }
                    Progress::Players => {
                        ui.add(ProgressBar::new(0.6).text(format!("{progress:?}")));
                    }
                    Progress::Towns => {
                        ui.add(ProgressBar::new(0.8).text(format!("{progress:?}")));
                    }
                    Progress::Islands => {
                        ui.add(ProgressBar::new(1.0).text(format!("{progress:?}")));
                    }
                }
            });
        });
    }

    #[allow(clippy::too_many_lines)] // UI Code, am I right, hahah
    fn ui_init(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ui_menu(ctx, frame);
        self.ui_sidepanel(ctx);
        self.ui_map(ctx);
    }
}

impl eframe::App for View {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // enable screen reader support on the web
        ctx.options_mut(|o| o.screen_reader = true);

        // allow the user to zoom in and out
        // https://docs.rs/egui/latest/egui/gui_zoom/fn.zoom_with_keyboard_shortcuts.html
        if !frame.is_web() {
            egui::gui_zoom::zoom_with_keyboard_shortcuts(ctx, frame.info().native_pixels_per_point);
        }

        // make sure we process messages from the backend every once in a while
        ctx.request_repaint_after(Duration::from_millis(500));

        // process any messages that came in from the backend since the last frame
        while let Ok(message) = self.channel_presenter_rx.try_recv() {
            println!("Got Message from Model to View: {message}");
            match message {
                MessageToView::GotServer => {
                    self.ui_state = State::Show;
                    self.channel_presenter_tx
                        .send(MessageToModel::FetchAll)
                        .expect("Failed to send message to model: FetchAll");
                    self.channel_presenter_tx
                        .send(MessageToModel::FetchGhosts)
                        .expect("Failed to send message to model: FetchGhosts");

                    // ensure the towns in the selection are fetched anew after loading the data from the server.
                    // If we don't do this the selection may become stale and show towns from server ab12 on a
                    // map that is otherwise pulled from server cd34
                    for selection in &mut self.ui_data.selections {
                        selection.towns = Arc::new(Vec::new());
                        selection.refresh(&self.channel_presenter_tx);
                    }
                }
                MessageToView::TownListForSelection(selection, town_list) => {
                    self.ui_state = State::Show;
                    let optional_selection = self
                        .ui_data
                        .selections
                        .iter_mut()
                        .find(|element| *element == selection);
                    if let Some(selection) = optional_selection {
                        selection.towns = town_list;
                        selection.state = SelectionState::Finished;
                    } else {
                        eprintln!("No existing selection found for {selection}");
                    }
                }
                MessageToView::ValueListForConstraint(constraint, selection, towns) => {
                    self.ui_state = State::Show;
                    let optional_selection = self
                        .ui_data
                        .selections
                        .iter_mut()
                        .find(|element| *element == selection);
                    if let Some(selection) = optional_selection {
                        let optional_constraint =
                            selection.constraints.iter_mut().find(|c| **c == constraint);
                        if let Some(constraint) = optional_constraint {
                            constraint.drop_down_values = Some(towns);
                        } else {
                            eprintln!(
                                "No existing constraint {constraint} found in selection {selection}"
                            );
                        }
                    } else {
                        eprintln!("No existing selection found for {selection}");
                    }
                }
                MessageToView::AllTowns(towns) => {
                    self.ui_state = State::Show;
                    self.ui_data.all_towns = towns;
                }
                MessageToView::GhostTowns(towns) => {
                    self.ui_state = State::Show;
                    self.ui_data.ghost_towns = towns;
                }
                MessageToView::Loading(progress) => {
                    self.ui_state = State::Uninitialized(progress);
                }
                MessageToView::BackendCrashed(_err) => {
                    // technically we don't need to remove the displayed stuff yet. The data that
                    // is already loaded can persist. It's just that the user can't fetch any new data
                    // from the backend, so a warning about that should be fine.
                    self.ui_state = State::Uninitialized(Progress::BackendCrashed);
                }
                MessageToView::FoundSavedDatabases(list_of_paths) => {
                    self.ui_data.saved_db = list_of_paths;
                }
                MessageToView::RemovedDuplicateFiles(removed_dbs) => {
                    for saved_dbs in self.ui_data.saved_db.values_mut() {
                        saved_dbs.retain(|saved_db| !removed_dbs.contains(saved_db));
                    }
                }
            }
        }

        let state = self.ui_state.clone();
        match state {
            State::Uninitialized(progress) => self.ui_uninitialized(ctx, frame, progress),
            State::Show => self.ui_init(ctx, frame),
        }
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        let serde_result = serde_yaml::to_string(&self.ui_data);
        match serde_result {
            Ok(res) => {
                storage.set_string(eframe::APP_KEY, res);
                storage.flush();
            }
            Err(err) => {
                eprintln!("Failed to convert ui data to serialized format! {err}");
            }
        };
    }
}
