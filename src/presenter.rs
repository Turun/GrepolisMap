use anyhow::Context;
use eframe::epaint::ahash::HashMap;

use crate::message::{MessageToModel, MessageToView};
use crate::model::download::Database;
use crate::model::Model;
use crate::towns::Constraint;
use std::sync::mpsc;
use std::time::Duration;

pub struct Presenter {
    model: Model,
    channel_tx: mpsc::Sender<MessageToView>,
    channel_rx: mpsc::Receiver<MessageToModel>,
}

impl Presenter {
    pub fn new(rx: mpsc::Receiver<MessageToModel>, tx: mpsc::Sender<MessageToView>) -> Self {
        Self {
            model: Model::Uninitialized,
            channel_tx: tx,
            channel_rx: rx,
        }
    }

    /// Given a Result<MessageToView>, send it to the View if it is ok. If the sending
    /// fails, output to stderr with the message given in `error_channel`. If the given
    /// message result is error, simply output it to stderr.
    fn send_to_view(&self, msg_opt: anyhow::Result<MessageToView>, error_channel: String) {
        match msg_opt {
            Ok(msg) => {
                let res = self.channel_tx.send(msg).context(error_channel);
                if let Err(err) = res {
                    eprintln!("{err}");
                }
            }
            Err(err) => {
                eprintln!("{err}");
                let res = self
                    .channel_tx
                    .send(MessageToView::BackendCrashed(err))
                    .context(error_channel);
                if let Err(err) = res {
                    eprintln!("We crashed so hard we couldn't even tell the frontend we did! Error: {err}");
                }
            }
        }
    }

    #[allow(clippy::too_many_lines)] // processing all variants of incoming messages simply needs a lot of lines
    /// Start the service that handles incoming messages, calls the appropriate backend code and sends the resutls to the view
    pub fn start(&mut self) {
        for message in &self.channel_rx {
            println!("Got Message from View to Model: {message}");
            match message {
                MessageToModel::SetServer(server, ctx) => {
                    // TODO: automatically save each db we load (with timestamp) and let the user choose previous versions.
                    let db_result = Database::create_for_world(&server.id, &self.channel_tx, &ctx);
                    match db_result {
                        Ok(db) => {
                            self.model = Model::Loaded {
                                db,
                                ctx,
                                cache_strings: HashMap::default(),
                                cache_towns: HashMap::default(),
                            };
                            self.send_to_view(
                                Ok(MessageToView::GotServer),
                                String::from("Failed to send message 'got server'"),
                            );
                        }
                        Err(err) => {
                            self.model = Model::Uninitialized;
                            self.send_to_view(
                                Ok(MessageToView::BackendCrashed(err)),
                                String::from("Failed to send crash message to view"),
                            );
                        }
                    }
                }
                MessageToModel::FetchAll => {
                    let towns = self.model.get_all_towns();
                    let msg = towns.map(MessageToView::AllTowns);
                    self.send_to_view(msg, String::from("Failed to send all town list to view"));
                }
                MessageToModel::FetchGhosts => {
                    let towns = self.model.get_ghost_towns();
                    let msg = towns.map(MessageToView::GhostTowns);
                    self.send_to_view(msg, String::from("Failed to send ghost town list to view"));
                }
                MessageToModel::FetchTowns(selection) => {
                    // TODO at the moment, when the user types something into a drop down list, we refetch the
                    // possible drop down values for all other drop down boxes as well. And since the box the
                    // user types in changed its content, the ddvlists of the other ddbs isn't even cached!
                    // Additionally, the ddvlist of the ddb the user is currently typing in is reset with every
                    // key stroke, making it painfully slow to get the ddv list suggestions. We need to fix that.
                    // Getting the ddvlist of the ddb the user is currently typing in has the highest priority!
                    // All other ddvlists can wait.
                    // For now, the list of filled constraints it reversed, so that - assuming the user add constraints
                    // at the bottom in the ui - the relevant results are sent back first.
                    // Ideally in the future we would process the currently edited ddb first (likely cached anyway),
                    // then we check if the currently edited ddv is a == or <> type. If it is and the user is currently
                    // changing its value, the db will probably not return any results for the other ddvlists anyway
                    // ("New Powe" is not an ally name, it needs to be complete before it matches anything in the db).
                    // Only if it is of type <= or >= does it make sense to fetch new ddvlists for the other ddb as well.

                    // a list of filled constraints. For each one, filter the ddv list by all _other_ filled constratins
                    let filled_constraints: Vec<Constraint> = selection
                        .constraints
                        .iter()
                        .rev()
                        .filter(|c| !c.value.is_empty())
                        .map(Constraint::partial_clone)
                        .collect();

                    // a list of empty constraints. Filter the ddv list by all non empty constraints
                    let empty_constraints: Vec<&Constraint> = selection
                        .constraints
                        .iter()
                        .filter(|c| c.value.is_empty())
                        .collect();

                    let towns = self.model.get_towns_for_constraints(&filled_constraints);
                    let msg = towns
                        .map(|t| MessageToView::TownListForSelection(selection.partial_clone(), t));
                    self.send_to_view(msg, String::from("Failed to send town list to view"));

                    // filled constraints
                    if filled_constraints.is_empty() {
                        // nothing
                    } else if filled_constraints.len() == 1 {
                        let c = filled_constraints[0].partial_clone();
                        let c_towns = self.model.get_names_for_constraint_type(c.constraint_type);
                        let msg = c_towns.map(|t| {
                            MessageToView::ValueListForConstraint(c, selection.partial_clone(), t)
                        });
                        self.send_to_view(msg, String::from("Failed to send town list to view"));
                    } else {
                        // for each constraint, make a list of all other filled constraints and get the ddv list filtered by those
                        for (i, c) in filled_constraints.iter().enumerate() {
                            let mut other_constraints = filled_constraints.clone();
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
                            self.send_to_view(
                                msg,
                                String::from("Failed to send town list to view"),
                            );
                        }
                    }

                    // empty constraints
                    if !empty_constraints.is_empty() {
                        for c in empty_constraints {
                            let c_towns =
                                self.model.get_names_for_constraint_type_with_constraints(
                                    c.constraint_type,
                                    &filled_constraints,
                                );
                            let msg = c_towns.map(|t| {
                                MessageToView::ValueListForConstraint(
                                    c.partial_clone(),
                                    selection.partial_clone(),
                                    t,
                                )
                            });
                            self.send_to_view(
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
        }
    }
}
