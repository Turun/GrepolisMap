mod data;
pub(crate) mod dropdownbox;
mod map;
mod menu;
pub(crate) mod preferences;
mod selectable_label;
mod sidepanel;

use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::presenter::Presenter;
use crate::presenter::PresenterReady;
use crate::selection::TownSelection;
#[cfg(not(target_arch = "wasm32"))]
use crate::storage;
use crate::telemetry;
use crate::view::data::Data;
#[cfg(target_arch = "wasm32")]
use crate::wasm_utils;
use eframe::Storage;
use egui::{FontData, ProgressBar, RichText, Ui};
use preferences::Telemetry;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

#[cfg(target_arch = "wasm32")]
use log::info;

#[derive(Clone, Copy)]
pub enum Change {
    Add,
    MoveUp(usize),
    Remove(usize),
    MoveDown(usize),
}

// TODO: the InSitu type is no longer relevant. We need to refactor the code and get rid of it.
pub enum Refresh {
    Complete,
    InSitu(HashSet<EmptyConstraint>),
    None,
}

#[derive(Debug, Clone)]
// Regarding Progress::BackendCrashed: technically we don't need to remove the displayed
// stuff yet and could keep the ui state as initialized. The data that is already loaded
// can persist. It's just that the user can't fetch any new data from the backend, so a
// warning about that should be fine.
pub enum Progress {
    None,
    BackendCrashed(String),
    Fetching,
    LoadingFile,
}

#[derive(Debug, Clone)]
pub enum State {
    Uninitialized(Progress),
    Show,
}

pub struct View {
    presenter: Presenter,
    ui_state: State,
    ui_data: Data,
}

impl View {
    #[allow(clippy::needless_pass_by_value)]
    fn setup(cc: &eframe::CreationContext) -> Self {
        let mut re = Self {
            presenter: Presenter::default(),
            ui_state: State::Uninitialized(Progress::None),
            ui_data: Data::default(),
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

        // load saved app data from disk
        if let Some(storage) = cc.storage {
            re.ui_data = if let Some(text) = storage.get_string(crate::APP_KEY) {
                // println!("{}", text);
                telemetry::event_stored_config(re.ui_data.preferences.telemetry, &text);
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

        re.presenter
            .set_max_cache_size(re.ui_data.preferences.cache_size);

        // start checking the latest version in the background. Will pop up a notification window if there is a newer version available
        // noop on wasm
        match re.ui_data.preferences.telemetry {
            Telemetry::All | Telemetry::OnlyVersionCheck => {
                telemetry::get_latest_version();
            }
            Telemetry::Nothing => {}
        }

        // TODO
        // self.channel_presenter_tx
        //     .send(MessageToModel::AutoDeleteTime(data.preferences.auto_delete_time))
        //     .expect("Failed to send message to backend: Discover Saved Databases");

        // apply preferences before returning
        re.ui_data
            .apply_darkmode(&cc.egui_ctx, re.ui_data.preferences.darkmode);
        re.ui_data.preferences.language.apply();

        //  and refresh list of saved dbs (which are not saved across app restarts)
        #[cfg(not(target_arch = "wasm32"))]
        {
            re.ui_data.saved_db = storage::get_list_of_saved_dbs();
        }

        #[cfg(target_arch = "wasm32")]
        {
            // TODO: the native app should also be able to load and share links.

            // TODO: state management - when should we save the selection state and when is it supposed to be treated
            // as temporary? I imagine a scenario where users usually have their base map, but alliances also share
            // interesting map filters. The users personal settings should not be overwritten just because they clicked
            // on a link and closed their browser tabs in the wrong order (the last tab closed will overwrite). I don't
            // want to introduce an extra screen just for that though. We could differentiate based on whether the users
            // came on the site via a direct link, or via the gmap.turun.de blank link. Blank -> save settings, full
            // link -> do not save settings. But that means that users would have to manually duplicate interesting
            // filters by hand - they were shared via link so they are not persisted.

            let (opt_url, opt_server_id, opt_selections) = wasm_utils::parse_current_url();
            info!("{opt_url:?} {opt_server_id:?}, {opt_selections:?}");
            re.ui_data.url = opt_url;
            if let Some(selections) = opt_selections {
                info!("URL contained info on selections, loading those");
                re.ui_data.selections = selections.iter().map(|ets| ets.fill()).collect();
            } else {
                info!("URL contained no info on selections");
            }
            if let Some(server_id) = opt_server_id {
                info!("URL contained info on server, loading data for it now");
                re.ui_data.server_id = server_id;

                // trigger server reload
                re.reload_server();
                // tell the backend to fetch data from the server
                re.presenter.load_server(re.ui_data.server_id.clone());
                re.ui_state = State::Uninitialized(Progress::Fetching);
            } else {
                info!("URL contained no info on server");
            }
        }

        re
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new_and_start() {
        // Redirect `log` message to `console.log` and friends:
        eframe::WebLogger::init(log::LevelFilter::Debug).ok();

        let web_options = eframe::WebOptions::default();

        wasm_bindgen_futures::spawn_local(async {
            let document = web_sys::window()
                .expect("No window")
                .document()
                .expect("No document");

            let start_result = eframe::WebRunner::new()
                .start(
                    "the_canvas_id", // canvas,
                    web_options,
                    Box::new(|cc| Box::new(View::setup(cc))),
                )
                .await;

            // Remove the loading text and spinner:
            if let Some(loading_text) = document.get_element_by_id("loading_text") {
                match start_result {
                    Ok(()) => {
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
    pub fn new_and_start() {
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
            Box::new(|cc| Box::new(View::setup(cc))),
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

        for selection in &mut self.ui_data.selections {
            selection.towns = Arc::new(Vec::new());
        }

        telemetry::event_load_server(self.ui_data.preferences.telemetry, &self.ui_data.server_id);
        // the selections are invalidated after the backend sends "got server"
    }

    fn ui_server_input(&mut self, ui: &mut Ui) {
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

        // on native show a button to (re)load the data that was last fetched.
        // Partially also there to serve as a note from what time the map data is from.
        #[cfg(not(target_arch = "wasm32"))]
        {
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
                    // change self.ui_data
                    self.reload_server();
                    // tell the backend to fetch data from the server
                    // this cannot be done in the normal chunk of messages, it needs to be triggered before the normal round of messages
                    self.presenter.load_server_from_file(saved_db);
                    self.ui_state = State::Uninitialized(Progress::LoadingFile);
                }
            }
        }

        if should_load_server {
            // change self.ui_data
            self.reload_server();
            // tell the backend to fetch data from the server
            // this cannot be done in the normal chunk of messages, it needs to be triggered before the normal round of messages
            self.presenter.load_server(self.ui_data.server_id.clone());
            self.ui_state = State::Uninitialized(Progress::Fetching);
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
                self.ui_server_input(ui);
                match progress {
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
                    Progress::Fetching => {
                        ui.add(
                            ProgressBar::new(self.presenter.loading_progress())
                                .text("Loading API data...".to_string()),
                        );
                    }
                    Progress::LoadingFile => {
                        ui.add(ProgressBar::new(0.5).text("Loading data from file...".to_string()));
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

        let presenter_ready_for_requests = self.presenter.ready_for_requests();
        // should we do anything special?
        match &presenter_ready_for_requests {
            Ok(PresenterReady::AlwaysHasBeen) => {}
            Ok(PresenterReady::WaitingForAPI) => {
                // still waiting for the API to respond. Make sure to check back in soon
                ctx.request_repaint_after(Duration::from_millis(50));
            }
            Ok(PresenterReady::NewlyReady) => {
                // trigger all the data refreshes that are required when loading new data
                self.ui_state = State::Show;
                self.ui_data.ghost_towns = self.presenter.get_ghost_towns();
                self.ui_data.all_towns = self.presenter.get_all_towns();

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
                    let result = selection.refresh_self(
                        &mut self.presenter,
                        &HashSet::new(),
                        &all_selections,
                    );
                    if let Err(err) = result {
                        self.ui_state =
                            State::Uninitialized(Progress::BackendCrashed(format!("{err:?}")));
                    }
                }

                // also refresh which SavedDBs are present. If we keep the *api response saving* in a
                // separate thread this refresh will still miss the latest response (because it will
                // take a while for the thread to save it). Nevertheless this is the best place I
                // can think of at the moment to put it.
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.ui_data.saved_db = storage::get_list_of_saved_dbs();
                }
            }
            Err(err) => {
                // crashed when trying to convert API Response into our backend data structure
                eprintln!("Backend Crashed with the following error:\n{err}");
                self.ui_state = State::Uninitialized(Progress::BackendCrashed(format!("{err}")));
            }
        }

        // the above is book keeping. Now we call the rendering code.
        let state = self.ui_state.clone();
        match state {
            State::Uninitialized(progress) => self.ui_uninitialized(ctx, frame, progress),
            State::Show => self.ui_init(ctx, frame),
        }

        #[cfg(target_arch = "wasm32")]
        {
            let this_states_url = wasm_utils::state_to_url_string(
                Some(&self.ui_data.server_id),
                Some(&self.ui_data.selections),
            );

            if self.ui_data.url != Some(this_states_url.clone()) {
                wasm_utils::set_current_url(&this_states_url);
                info!("Set url to:\n{this_states_url}");
                self.ui_data.url = Some(this_states_url);
            }
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
