mod data;
pub(crate) mod dropdownbox;
mod map;
mod menu;
pub(crate) mod preferences;
mod selectable_label;
mod sidepanel;

use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::message::{MessageToModel, MessageToServer, MessageToView, Progress, Server};
use crate::selection::{SelectionState, TownSelection};
use crate::view::data::Data;
use eframe::Storage;
use egui::{FontData, ProgressBar, RichText, Ui};
use std::collections::HashSet;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

#[derive(Clone, Copy)]
pub enum Change {
    Add,
    MoveUp(usize),
    Remove(usize),
    MoveDown(usize),
}

pub enum Refresh {
    Complete,
    InSitu(HashSet<EmptyConstraint>),
    None,
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
    #[allow(clippy::needless_pass_by_value)]
    fn setup(
        cc: &eframe::CreationContext,
        rx: mpsc::Receiver<MessageToView>,
        tx: mpsc::Sender<MessageToModel>,
        telemetry_tx: mpsc::Sender<MessageToServer>,
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
            re.ui_data = if let Some(text) = storage.get_string(crate::APP_KEY) {
                // println!("{}", text);
                let _result = telemetry_tx.send(MessageToServer::StoredConfig(text.clone()));
                serde_yaml::from_str(&text).unwrap_or_else(|err| {
                    eprintln!("Failed to read saved config as YAML: {err}");
                    Data::default()
                })
            } else {
                println!("No previously saved preferences found");
                Data::default()
            };
        } else {
            println!("No persistence storage configured");
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

        re.ui_data.preferences.language.apply();

        re
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new_and_start(
        rx: mpsc::Receiver<MessageToView>,
        tx: mpsc::Sender<MessageToModel>,
        telemetry_tx: mpsc::Sender<MessageToServer>,
    ) {
        use eframe::wasm_bindgen::JsCast as _;

        // Redirect `log` message to `console.log` and friends:
        eframe::WebLogger::init(log::LevelFilter::Debug).ok();

        let web_options = eframe::WebOptions::default();

        wasm_bindgen_futures::spawn_local(async {
            let document = web_sys::window()
                .expect("No window")
                .document()
                .expect("No document");

            let canvas = document
                .get_element_by_id("the_canvas_id")
                .expect("Failed to find the_canvas_id")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("the_canvas_id was not a HtmlCanvasElement");

            let start_result = eframe::WebRunner::new()
                .start(
                    canvas,
                    web_options,
                    Box::new(|cc| Ok(Box::new(View::setup(cc, rx, tx, telemetry_tx)))),
                )
                .await;

            // Remove the loading text and spinner:
            if let Some(loading_text) = document.get_element_by_id("loading_text") {
                match start_result {
                    Ok(_) => {
                        loading_text.remove();
                    }
                    Err(e) => {
                        loading_text.set_inner_html(
                            "<p> The app has crashed. See the developer console for details. </p>",
                        );
                        panic!("Failed to start eframe: {e:?}");
                    }
                }
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_and_start(
        rx: mpsc::Receiver<MessageToView>,
        tx: mpsc::Sender<MessageToModel>,
        telemetry_tx: mpsc::Sender<MessageToServer>,
    ) {
        let native_options = eframe::NativeOptions {
            // defaults to window title, but we include the version in the window title. Since
            // it should stay the same across version changes we give it a fixed value here.
            viewport: egui::ViewportBuilder::default().with_app_id("Turun Map"),
            ..eframe::NativeOptions::default()
        };
        let version = env!("CARGO_PKG_VERSION");
        eframe::run_native(
            &format!("Turun Map {version}"),
            native_options,
            Box::new(|cc| Ok(Box::new(View::setup(cc, rx, tx, telemetry_tx)))),
        )
        .expect("Eframe failed!");
    }

    fn reset_saved_preferences(frame: &mut eframe::Frame) {
        if let Some(storage) = frame.storage_mut() {
            storage.set_string(crate::APP_KEY, String::new());
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
            ui.label(t!("sidepanel.header.server_id"));
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
            .add(egui::Button::new(t!("sidepanel.header.load_data")))
            .clicked()
        {
            should_load_server = true;
        }

        let most_recently_loaded = self
            .ui_data
            .saved_db
            .iter()
            .flat_map(|(server_id, saved_dbs)| {
                saved_dbs
                    .iter()
                    .map(|saved_db| (server_id.clone(), saved_db.clone()))
            })
            .max_by_key(|(_, saved_db)| saved_db.date);

        if let Some((server_id, saved_db)) = most_recently_loaded {
            if ui
                .button(t!(
                    "sidepanel.header.open_recent",
                    server_id = server_id,
                    db = saved_db
                ))
                .clicked()
            {
                self.ui_data.server_id = server_id;
                self.reload_server();
                self.channel_presenter_tx
                    .send(MessageToModel::LoadDataFromFile(saved_db.path, ctx.clone()))
                    .expect("Failed to send message to Model");
                self.ui_state = State::Uninitialized(Progress::None);
            }
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
                    // TODO adjust this once the backend has moved from sql to rust
                    Progress::None => {}
                    Progress::BackendCrashed(stringified_reason) => {
                        ui.label(
                            RichText::new(t!(
                                "sidepanel.loading.db_crashed",
                                reason = stringified_reason
                            ))
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
                    Progress::Islands => {
                        ui.add(ProgressBar::new(0.6).text(format!("{progress:?}")));
                    }
                    Progress::Players => {
                        ui.add(ProgressBar::new(0.8).text(format!("{progress:?}")));
                    }
                    Progress::Towns => {
                        ui.add(ProgressBar::new(1.0).text(format!("{progress:?}")));
                    }
                }
            });
        });
    }

    fn ui_init(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ui_menu(ctx, frame);
        self.ui_sidepanel(ctx);
        self.ui_map(ctx);
    }
}

impl eframe::App for View {
    #[allow(clippy::too_many_lines)]
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // enable screen reader support on the web
        ctx.options_mut(|o| o.screen_reader = true);

        // make sure we process messages from the backend every once in a while
        ctx.request_repaint_after(Duration::from_millis(500));

        // process any messages that came in from the backend since the last frame
        while let Ok(message) = self.channel_presenter_rx.try_recv() {
            // println!("Got Message from Model to View: {message}");
            match message {
                MessageToView::VersionInfo(server_version, message) => {
                    // TODO preferences -> disable telemetry
                    #[cfg(not(target_arch = "wasm32"))]
                    // not on wasm, because this code block uses native_dialog
                    {
                        let this_version = env!("CARGO_PKG_VERSION");
                        let _handle = thread::spawn(move || {
                            let _result = native_dialog::MessageDialog::new()
                                .set_title(&t!("menu.update_notice.title"))
                                .set_text(&t!(
                                    "menu.update_notice.content",
                                    user_version = this_version,
                                    server_version = server_version,
                                    message = message
                                ))
                                .set_type(native_dialog::MessageType::Info)
                                .show_alert();
                        });
                    }
                }
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
                    let all_selections: Vec<EmptyTownSelection> = self
                        .ui_data
                        .selections
                        .iter()
                        .map(TownSelection::partial_clone)
                        .collect();
                    for selection in &mut self.ui_data.selections {
                        selection.towns = Arc::new(Vec::new());
                        selection.refresh_self(
                            &self.channel_presenter_tx,
                            HashSet::new(),
                            &all_selections,
                        );
                    }
                }
                MessageToView::TownListForSelection(selection, town_list) => {
                    self.ui_state = State::Show;
                    let optional_selection = self
                        .ui_data
                        .selections
                        .iter_mut()
                        .find(|element| element.hidden_id == selection.hidden_id);
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
                        .find(|element| element.hidden_id == selection.hidden_id);
                    if let Some(selection) = optional_selection {
                        let optional_constraint =
                            selection.constraints.iter_mut().find(|c| **c == constraint);
                        if let Some(constraint) = optional_constraint {
                            constraint.drop_down_values = Some(towns);
                        } else {
                            eprintln!(
                                "No existing constraint {constraint} found in selection {}",
                                selection.partial_clone()
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
                MessageToView::BackendCrashed(err) => {
                    // technically we don't need to remove the displayed stuff yet. The data that
                    // is already loaded can persist. It's just that the user can't fetch any new data
                    // from the backend, so a warning about that should be fine.
                    eprintln!("Backend Crashed with the following error:\n{err:?}");
                    self.ui_state =
                        State::Uninitialized(Progress::BackendCrashed(format!("{err:?}")));
                }
                MessageToView::FoundSavedDatabases(list_of_paths) => {
                    self.ui_data.saved_db = list_of_paths;
                }
                MessageToView::RemovedDatabases(removed_dbs) => {
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
                storage.set_string(crate::APP_KEY, res);
                storage.flush();
            }
            Err(err) => {
                eprintln!("Failed to convert ui data to serialized format! {err}");
            }
        };
    }
}
