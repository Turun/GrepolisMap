use anyhow::Context;
use eframe::epaint::ahash::HashMap;

use crate::message::{MessageToModel, MessageToView};
use crate::model::database::Database;
use crate::model::Model;
use crate::storage;
use crate::towns::Constraint;
use crate::view::preferences::CacheSize;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Given a Result<MessageToView>, send it to the View if it is ok. If the sending
/// fails, output to stderr with the message given in `error_channel`. If the given
/// message result is error, simply output it to stderr.
fn send_to_view(
    tx: &mpsc::Sender<MessageToView>,
    msg_opt: anyhow::Result<MessageToView>,
    error_channel: String,
) {
    match msg_opt {
        Ok(msg) => {
            let res = tx.send(msg).context(error_channel);
            if let Err(err) = res {
                eprintln!("{err:?}");
            }
        }
        Err(err) => {
            let res = tx
                .send(MessageToView::BackendCrashed(err))
                .context(error_channel);
            if let Err(err) = res {
                eprintln!(
                    "We crashed so hard we couldn't even tell the frontend we did! Error: {err:?}"
                );
            }
        }
    }
}

pub struct Presenter {
    model: Model,
    max_cache_size: CacheSize,
    channel_tx: mpsc::Sender<MessageToView>,
    channel_rx: mpsc::Receiver<MessageToModel>,
}

impl Presenter {
    pub fn new(rx: mpsc::Receiver<MessageToModel>, tx: mpsc::Sender<MessageToView>) -> Self {
        Self {
            model: Model::Uninitialized,
            max_cache_size: CacheSize::Normal,
            channel_tx: tx,
            channel_rx: rx,
        }
    }

    #[allow(clippy::too_many_lines)] // processing all variants of incoming messages simply needs a lot of lines
    /// Start the service that handles incoming messages, calls the appropriate backend code and sends the resutls to the view
    pub fn start(&mut self) {
        let mut spawned_threads = Vec::new();

        for message in &self.channel_rx {
            println!("Got Message from View to Model: {message}");
            match message {
                MessageToModel::MaxCacheSize(x) => {
                    self.max_cache_size = x;
                }
                MessageToModel::DiscoverSavedDatabases => {
                    let thread_tx = self.channel_tx.clone();
                    let handle = thread::spawn(move || {
                        let dbs = storage::get_list_of_saved_dbs();
                        send_to_view(
                            &thread_tx,
                            Ok(MessageToView::FoundSavedDatabases(dbs)),
                            String::from("Failed to send list of saved dbs to View"),
                        );
                    });
                    spawned_threads.push(handle);
                }
                MessageToModel::LoadDataFromFile(path, ctx) => {
                    let db_result = Database::load_from_file(&path);
                    match db_result {
                        Ok(db) => {
                            self.model = Model::Loaded {
                                db,
                                ctx,
                                cache_strings: HashMap::default(),
                                cache_towns: HashMap::default(),
                            };
                            send_to_view(
                                &self.channel_tx,
                                Ok(MessageToView::GotServer),
                                String::from("Failed to send message 'got server'"),
                            );
                        }
                        Err(err) => {
                            self.model = Model::Uninitialized;
                            send_to_view(
                                &self.channel_tx,
                                Ok(MessageToView::BackendCrashed(err)),
                                String::from("Failed to send crash message to view"),
                            );
                        }
                    }
                }
                MessageToModel::SetServer(server, ctx) => {
                    let db_path = storage::get_new_db_filename(&server.id);
                    let db_result = Database::create_for_world(
                        &server.id,
                        db_path.as_deref(),
                        &self.channel_tx,
                        &ctx,
                    );
                    // TODO: if the db we just created is identical to a previously saved file we should get rid of one of them.
                    //       optionally this can be done as a background process. We could also leave the just created db alone, no matter what
                    //       and only touch those that had been created in previous runs of the program
                    match db_result {
                        Ok(db) => {
                            self.model = Model::Loaded {
                                db,
                                ctx,
                                cache_strings: HashMap::default(),
                                cache_towns: HashMap::default(),
                            };
                            send_to_view(
                                &self.channel_tx,
                                Ok(MessageToView::GotServer),
                                String::from("Failed to send message 'got server'"),
                            );
                        }
                        Err(err) => {
                            self.model = Model::Uninitialized;
                            send_to_view(
                                &self.channel_tx,
                                Ok(MessageToView::BackendCrashed(err)),
                                String::from("Failed to send crash message to view"),
                            );

                            // if we failed halfway during the creation of our db, we need to remove the unfinished db from the filesystem
                            if let Some(path) = db_path {
                                let _result = storage::remove_db(&path);
                            }
                        }
                    }
                }
                MessageToModel::FetchAll => {
                    let towns = self.model.get_all_towns();
                    let msg = towns.map(MessageToView::AllTowns);
                    send_to_view(
                        &self.channel_tx,
                        msg,
                        String::from("Failed to send all town list to view"),
                    );
                }
                MessageToModel::FetchGhosts => {
                    let towns = self.model.get_ghost_towns();
                    let msg = towns.map(MessageToView::GhostTowns);
                    send_to_view(
                        &self.channel_tx,
                        msg,
                        String::from("Failed to send ghost town list to view"),
                    );
                }
                MessageToModel::FetchTowns(selection, constraints_edited) => {
                    // a list of filled constraints that are not being edited. For each one, filter the ddv list by all _other_ filled, unedited constratins
                    let constraints_filled_not_edited: Vec<Constraint> = selection
                        .constraints
                        .iter()
                        .rev()
                        .filter(|c| !c.value.is_empty())
                        .filter(|c| !constraints_edited.contains(c))
                        .map(Constraint::partial_clone)
                        .collect();

                    // a list of filled constraints. For each one, filter the ddv list by all _other_ filled constratins
                    let constraints_filled_all: Vec<Constraint> = selection
                        .constraints
                        .iter()
                        .rev()
                        .filter(|c| !c.value.is_empty())
                        .map(Constraint::partial_clone)
                        .collect();

                    // a list of empty constraints. Filter the ddv list by all non empty constraints
                    let constraints_empty: Vec<&Constraint> = selection
                        .constraints
                        .iter()
                        .filter(|c| c.value.is_empty())
                        .filter(|c| !constraints_edited.contains(c))
                        .collect();

                    // The drop down values for the constraints currently being edited
                    for c in constraints_edited {
                        let towns = self.model.get_names_for_constraint_type_with_constraints(
                            c.constraint_type,
                            &constraints_filled_not_edited,
                        );
                        let msg = towns.map(|t| {
                            MessageToView::ValueListForConstraint(
                                c.partial_clone(),
                                selection.partial_clone(),
                                t,
                            )
                        });
                        send_to_view(
                            &self.channel_tx,
                            msg,
                            String::from("Failed to send town list for currently edited drop down"),
                        );
                    }

                    // Towns of this selection
                    let towns = self
                        .model
                        .get_towns_for_constraints(&constraints_filled_all);
                    let msg = towns
                        .map(|t| MessageToView::TownListForSelection(selection.partial_clone(), t));
                    send_to_view(
                        &self.channel_tx,
                        msg,
                        String::from("Failed to send town list to view"),
                    );

                    // drop down values for the empty constraints
                    if !constraints_empty.is_empty() {
                        for c in constraints_empty {
                            let c_towns =
                                self.model.get_names_for_constraint_type_with_constraints(
                                    c.constraint_type,
                                    &constraints_filled_all,
                                );
                            let msg = c_towns.map(|t| {
                                MessageToView::ValueListForConstraint(
                                    c.partial_clone(),
                                    selection.partial_clone(),
                                    t,
                                )
                            });
                            send_to_view(
                                &self.channel_tx,
                                msg,
                                String::from("Failed to send town list to view"),
                            );
                        }
                    }

                    // drop down values for the filled constraints
                    if constraints_filled_not_edited.is_empty() {
                        // nothing
                    } else if constraints_filled_not_edited.len() == 1 {
                        let c = constraints_filled_not_edited[0].partial_clone();
                        let c_towns = self.model.get_names_for_constraint_type(c.constraint_type);
                        let msg = c_towns.map(|t| {
                            MessageToView::ValueListForConstraint(c, selection.partial_clone(), t)
                        });
                        send_to_view(
                            &self.channel_tx,
                            msg,
                            String::from("Failed to send town list to view"),
                        );
                    } else {
                        // for each constraint, make a list of all other filled constraints and get the ddv list filtered by those
                        for (i, c) in constraints_filled_not_edited.iter().enumerate() {
                            // TODO: only a slight improvement, but we could select the drop down values
                            //  not by constrain with all other filled constraints, but instead by all other
                            //  filled constraints that do not, on their own, reduce the result list to zero.
                            //  What I mean by that is that, when the user has set an alliance name or player
                            //  name field to == and entered a partial name, that constraint will reduce the
                            //  ddvlist of all filled constraints to an empty list. But that is not the drop
                            //  down the user wants to see. They want to see the possible values of ddb xxx
                            //  to show the possible values, given the other useful constraints.
                            //  To implement this we will have to set other_constraints to a list of
                            //  filled_constraints minus the set of filled constraints that give no
                            //  result from the database.
                            let mut other_constraints = constraints_filled_all.clone();
                            let _this_constraint = other_constraints.swap_remove(i);

                            let c_towns =
                                self.model.get_names_for_constraint_type_with_constraints(
                                    c.constraint_type,
                                    &other_constraints,
                                );
                            let msg = c_towns.map(|t| {
                                MessageToView::ValueListForConstraint(
                                    c.partial_clone(),
                                    selection.partial_clone(),
                                    t,
                                )
                            });
                            send_to_view(
                                &self.channel_tx,
                                msg,
                                String::from("Failed to send town list to view"),
                            );
                        }
                    }
                }
            }

            // after we process a message, tell the UI to repaint
            // this helps a lot speeding things along, but sometimes the UI finished painting
            // before receiving the message. In that case it fulfilled the request_repaint here,
            // but goes to sleep before it can fulfill the message intent.
            self.model.request_repaint_after(Duration::from_millis(50));
            self.model.age_cache(self.max_cache_size.value());
        }

        for handle in spawned_threads {
            handle.join().expect("Failed to join extra backend thread");
        }
    }
}
