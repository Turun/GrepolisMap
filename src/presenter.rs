use anyhow::anyhow;
use eframe::epaint::ahash::HashMap;

use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::message::{MessageToModel, MessageToView, PresenterReady};
use crate::model::database::DataTable;
use crate::model::{APIResponse, Model};
use crate::storage;
use crate::view::preferences::CacheSize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Given a Result<MessageToView>, send it to the View if it is ok. If the sending
/// fails, output to stderr with the message given in `error_channel`. If the given
/// message result is error, simply output it to stderr.
fn send_to_view(
    tx: &mut Vec<MessageToView>,
    msg_opt: anyhow::Result<MessageToView>,
    _error_channel: String,
) {
    match msg_opt {
        Ok(msg) => {
            tx.push(msg);
        }
        Err(err) => {
            tx.push(MessageToView::BackendCrashed(format!("{err}")));
        }
    }
}

pub struct Presenter {
    model: Model,
    max_cache_size: CacheSize,
}

impl Presenter {
    pub fn new() -> Self {
        Self {
            model: Model::Uninitialized(Arc::new(Mutex::new(APIResponse::new(String::new())))),
            max_cache_size: CacheSize::Normal,
        }
    }

    /// Return all possible selection names that can be used in the `DropDownValues` for the
    /// Constraint. Returns None if the Constraint is not a IN/NOT IN type of constraint. In which
    /// case it is up to the caller to determine which drop down values are appropriate.
    fn possible_ddv_selections_or(
        constraint: &EmptyConstraint,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> Option<Arc<Vec<String>>> {
        constraint
            .referenced_selection()
            .map(|_referenced_selection| {
                Arc::new(
                    all_selections
                        .iter()
                        .map(|s| s.name.clone())
                        .filter(|name| name != &selection.name)
                        .filter(|name| {
                            let mut test_selection = selection.clone();
                            test_selection.constraints.push(EmptyConstraint {
                                constraint_type: crate::constraint::ConstraintType::PlayerName,
                                comparator: crate::constraint::Comparator::InSelection,
                                value: name.clone(),
                            });
                            !test_selection.contains_circular_reference(all_selections)
                        })
                        .collect(),
                )
            })
    }

    /// triggers the server loading, which is handled asynchronously
    /// This is deliberately its own method, because the self.model = Model::Uninit needs to be triggered before the
    /// normal message processing.
    pub fn load_server(&mut self, server: String) {
        let api_response = Arc::new(Mutex::new(APIResponse::new(server)));
        self.model = Model::Uninitialized(Arc::clone(&api_response));
        DataTable::get_api_results(Arc::clone(&api_response));
    }

    /// triggers the server loading, which is handled asynchronously
    /// This is deliberately its own method, because the self.model = Model::Uninit needs to be triggered before the
    /// normal message processing.
    pub fn load_server_from_file(&mut self, file: PathBuf) {
        let api_response = Arc::new(Mutex::new(APIResponse::empty()));
        self.model = Model::Uninitialized(Arc::clone(&api_response));
        APIResponse::load_from_file(file, api_response);
    }

    /// returns how many of the api requests already completed. i.e. 1/4 -> 0.25
    /// if model is not in the loading state we return a flat 1.0
    pub fn loading_progress(&self) -> f32 {
        match &self.model {
            Model::Uninitialized(arc) => arc.lock().unwrap().count_completed() as f32 / 4.0,
            Model::Loaded { .. } => 1.0,
        }
    }

    /// returns Some(true) if the model is initialized and the presenter can start answering requests.
    /// returns None if the backend crashed trying to parse the complete api response.
    /// returns Some(false) if the api data is still being fetched.
    pub fn ready_for_requests(&mut self) -> anyhow::Result<PresenterReady> {
        match &self.model {
            Model::Uninitialized(api_response) => {
                let api_response = api_response.lock().unwrap().clone();
                if api_response.is_complete() {
                    api_response.save_to_file(); // TODO: extract into extra thread. since this is only for native, we can simply use a thread (make sure to gate with feature though)
                    let db_path = api_response.filename.clone();
                    let db_result = DataTable::create_for_world(api_response);

                    match db_result {
                        Ok(db) => {
                            self.model = Model::Loaded {
                                db,
                                cache_strings: HashMap::default(),
                                cache_towns: HashMap::default(),
                            };
                            return Ok(PresenterReady::NewlyReady);
                        }
                        Err(err) => {
                            self.model = Model::Uninitialized(Arc::new(Mutex::new(
                                APIResponse::new(String::new()),
                            )));

                            // if we failed halfway during the creation of our db, we need to remove the unfinished db from the filesystem
                            if let Some(path) = db_path {
                                let _result = storage::remove_db(&path);
                            }

                            return Err(anyhow!("{err}"));
                        }
                    }
                } else {
                    return Ok(PresenterReady::WaitingForAPI);
                }
            }
            Model::Loaded { .. } => return Ok(PresenterReady::AlwaysHasBeen),
        }
    }

    #[allow(clippy::too_many_lines)] // processing all variants of incoming messages simply needs a lot of lines
    /// Start the service that handles incoming messages, calls the appropriate backend code and sends the resutls to the view
    pub fn process_messages(&mut self, messages: &[MessageToModel]) -> Vec<MessageToView> {
        let mut re = Vec::new();

        for message in messages {
            println!("Got Message from View to Model: {message}");
            match message {
                MessageToModel::MaxCacheSize(x) => {
                    self.max_cache_size = x.clone();
                }
                MessageToModel::LoadDataFromFile(path, ctx) => {
                    todo!("we had this in the SQL version, but it's still a TODO for the rust only version");
                    // let db_result = Database::load_from_file(&path);
                    // match db_result {
                    //     Ok(db) => {
                    //         self.model = Model::Loaded {
                    //             db,
                    //             ctx,
                    //             cache_strings: HashMap::default(),
                    //             cache_towns: HashMap::default(),
                    //         };
                    //         send_to_view(
                    //             &self.channel_tx,
                    //             Ok(MessageToView::GotServer),
                    //             String::from("Failed to send message 'got server'"),
                    //         );
                    //     }
                    //     Err(err) => {
                    //         self.model = Model::Uninitialized;
                    //         send_to_view(
                    //             &self.channel_tx,
                    //             Ok(MessageToView::BackendCrashed(err)),
                    //             String::from("Failed to send crash message to view"),
                    //         );
                    //     }
                    // }
                }
                MessageToModel::FetchAll => {
                    let towns = self.model.get_all_towns();
                    let msg = towns.map(MessageToView::AllTowns);
                    send_to_view(
                        &mut re,
                        msg,
                        String::from("Failed to send all town list to view"),
                    );
                }
                MessageToModel::FetchGhosts => {
                    let towns = self.model.get_ghost_towns();
                    let msg = towns.map(MessageToView::GhostTowns);
                    send_to_view(
                        &mut re,
                        msg,
                        String::from("Failed to send ghost town list to view"),
                    );
                }
                MessageToModel::FetchTowns(selection, constraints_edited, all_selections) => {
                    // a list of filled constraints that are not being edited. For each one, filter the ddv list by all _other_ filled, unedited constratins
                    let constraints_filled_not_edited: Vec<EmptyConstraint> = selection
                        .constraints
                        .iter()
                        .rev()
                        .filter(|c| !c.value.is_empty())
                        .filter(|c| !constraints_edited.contains(c))
                        .cloned()
                        .collect();

                    // a list of filled constraints. For each one, filter the ddv list by all _other_ filled constratins
                    let constraints_filled_all: Vec<EmptyConstraint> = selection
                        .constraints
                        .iter()
                        .rev()
                        .filter(|c| !c.value.is_empty())
                        .cloned()
                        .collect();

                    // a list of empty constraints. Filter the ddv list by all non empty constraints
                    let constraints_empty: Vec<&EmptyConstraint> = selection
                        .constraints
                        .iter()
                        .filter(|c| c.value.is_empty())
                        .filter(|c| !constraints_edited.contains(c))
                        .collect();

                    // The drop down values for the constraints currently being edited
                    for c in constraints_edited {
                        let possible_ddv =
                            Self::possible_ddv_selections_or(&c, &selection, &all_selections)
                                .ok_or(Err(0))
                                .or_else(|_error_value: Result<Arc<Vec<String>>, i32>| {
                                    self.model.get_names_for_constraint_with_constraints(
                                        &selection,
                                        c.constraint_type,
                                        &constraints_filled_not_edited,
                                        &all_selections,
                                    )
                                });
                        let msg = possible_ddv.map(|t| {
                            MessageToView::ValueListForConstraint(c.clone(), selection.clone(), t)
                        });
                        send_to_view(
                            &mut re,
                            msg,
                            String::from("Failed to send town list for currently edited drop down"),
                        );
                    }

                    // Towns of this selection
                    let towns = self.model.get_towns_for_constraints(
                        &selection,
                        &constraints_filled_all,
                        &all_selections,
                    );
                    let msg =
                        towns.map(|t| MessageToView::TownListForSelection(selection.clone(), t));
                    send_to_view(
                        &mut re,
                        msg,
                        String::from("Failed to send town list to view"),
                    );

                    // drop down values for the empty constraints
                    if !constraints_empty.is_empty() {
                        for c in constraints_empty {
                            let possible_ddv =
                                Self::possible_ddv_selections_or(c, &selection, &all_selections)
                                    .ok_or(Err(0))
                                    .or_else(|_error_value: Result<Arc<Vec<String>>, i32>| {
                                        self.model.get_names_for_constraint_with_constraints(
                                            &selection,
                                            c.constraint_type,
                                            &constraints_filled_all,
                                            &all_selections,
                                        )
                                    });
                            let msg = possible_ddv.map(|t| {
                                MessageToView::ValueListForConstraint(
                                    c.clone(),
                                    selection.clone(),
                                    t,
                                )
                            });
                            send_to_view(
                                &mut re,
                                msg,
                                String::from("Failed to send town list to view"),
                            );
                        }
                    }

                    // drop down values for the filled constraints
                    if constraints_filled_not_edited.is_empty() {
                        // nothing
                    } else if constraints_filled_not_edited.len() == 1 {
                        // only one constraint that is filled, but not edited -> no restrictions apply
                        let c = constraints_filled_not_edited[0].clone();
                        let possible_ddv =
                            Self::possible_ddv_selections_or(&c, &selection, &all_selections)
                                .ok_or(Err(0))
                                .or_else(|_error_value: Result<Arc<Vec<String>>, i32>| {
                                    self.model.get_names_for_constraint_type(c.constraint_type)
                                });
                        let msg = possible_ddv.map(|t| {
                            MessageToView::ValueListForConstraint(c, selection.clone(), t)
                        });
                        send_to_view(
                            &mut re,
                            msg,
                            String::from("Failed to send town list to view"),
                        );
                    } else {
                        // for each constraint, make a list of all other filled constraints and get the ddv list filtered by those
                        for c in constraints_filled_not_edited {
                            let mut other_constraints = constraints_filled_all.clone();
                            let index = other_constraints.iter().position(|x| x == &c);
                            if index.is_none() {
                                continue;
                            }
                            let index = index.unwrap();
                            let _this_constraint = other_constraints.swap_remove(index);

                            let possible_ddv =
                                Self::possible_ddv_selections_or(&c, &selection, &all_selections)
                                    .ok_or(Err(0))
                                    .or_else(|_error_value: Result<Arc<Vec<String>>, i32>| {
                                        self.model.get_names_for_constraint_with_constraints(
                                            &selection,
                                            c.constraint_type,
                                            &other_constraints,
                                            &all_selections,
                                        )
                                    });
                            let msg = possible_ddv.map(|t| {
                                MessageToView::ValueListForConstraint(
                                    c.clone(),
                                    selection.clone(),
                                    t,
                                )
                            });
                            send_to_view(
                                &mut re,
                                msg,
                                String::from("Failed to send town list to view"),
                            );
                        }
                    }
                }
            }

            self.model.age_cache(self.max_cache_size.value());
        }

        return re;
    }
}
