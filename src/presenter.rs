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
    pub fn start(&mut self) {
        for message in &self.channel_rx {
            println!("Got Message from View to Model: {}", message);
            //TODO multithreaded database. We only read, never write after the inital creation.
            // Could help a lot if we add town lists to each dropdown
            //TODO better error handling for the database. We should not let the model
            //  thread crash due to DB issues. In
            //  the worst case the user can just try to reload the data
            match message {
                MessageToModel::SetServer(server, ctx) => {
                    // TODO: automatically save each db we load and let the user choose previous versions.
                    let db = Database::create_for_world(&server.id, self.channel_tx.clone(), &ctx)
                        .unwrap();
                    self.model = Model::Loaded { db, ctx };
                    self.channel_tx
                        .send(MessageToView::GotServer)
                        .expect("Failed to send message 'got server'");
                }
                MessageToModel::FetchAll => {
                    let towns = self.model.get_all_towns();
                    self.channel_tx
                        .send(MessageToView::AllTowns(towns))
                        .expect("Failed to send all town list to view");
                }
                MessageToModel::FetchGhosts => {
                    let towns = self.model.get_ghost_towns();
                    self.channel_tx
                        .send(MessageToView::GhostTowns(towns))
                        .expect("Failed to send ghost town list to view");
                }
                MessageToModel::FetchDropDownValues(constraint_type) => {
                    let names = self.model.get_names_for_constraint_type(&constraint_type);
                    self.channel_tx
                        .send(MessageToView::DropDownValues(constraint_type, names))
                        .expect("Failed to send drop down value list to view");
                }
                MessageToModel::FetchTowns(selection) => {
                    let towns = self.model.get_towns_for_selection(&selection);
                    self.channel_tx
                        .send(MessageToView::TownListSelection(
                            selection.partial_clone(),
                            towns,
                        ))
                        .expect("Failed to send town list to view");

                    // a list of filled constraints. For each one, filter the ddv list by all _other_ filled constratins
                    let filled_constraints: Vec<&Constraint> = selection
                        .constraints
                        .iter()
                        .filter(|c| !c.value.is_empty())
                        .collect();

                    if filled_constraints.len() == 0 {
                        // nothing
                    } else if filled_constraints.len() == 1 {
                        let c = filled_constraints[0];
                        let constraint_towns =
                            self.model.get_names_for_constraint_type(&c.constraint_type);
                        self.channel_tx
                            .send(MessageToView::TownListConstraint(
                                c.partial_clone(),
                                selection.partial_clone(),
                                constraint_towns,
                            ))
                            .expect("Failed to send town list to view");
                    } else {
                        // for each constraint, make a list of all other filled constraints and get the ddv list filtered by those
                        for (i, c) in filled_constraints.iter().enumerate() {
                            let mut other_constraints = filled_constraints.clone();
                            let _this_constraint = other_constraints.swap_remove(i);

                            let constraint_towns =
                                self.model.get_names_for_constraint_type_with_constraints(
                                    &c.constraint_type,
                                    &filled_constraints,
                                );

                            self.channel_tx
                                .send(MessageToView::TownListConstraint(
                                    c.partial_clone(),
                                    selection.partial_clone(),
                                    constraint_towns,
                                ))
                                .expect("Failed to send town list to view");
                        }
                    }
                    // a list of empty constraints. Filter the ddv list by all non empty constraints
                    let empty_constraints: Vec<&Constraint> = selection
                        .constraints
                        .iter()
                        .filter(|c| c.value.is_empty())
                        .collect();
                    if empty_constraints.len() > 0 {
                        for c in empty_constraints {
                            let constraint_towns =
                                self.model.get_names_for_constraint_type_with_constraints(
                                    &c.constraint_type,
                                    &filled_constraints,
                                );

                            self.channel_tx
                                .send(MessageToView::TownListConstraint(
                                    c.partial_clone(),
                                    selection.partial_clone(),
                                    constraint_towns,
                                ))
                                .expect("Failed to send town list to view");
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
