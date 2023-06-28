use crate::message::{MessageToModel, MessageToView};
use crate::model::download::Database;
use crate::model::Model;
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

                    // TODO we need a way to set Constratin ddvalue lists, even when there is only one constraint in the selection.
                    // This will be the full list without any WHERE clauses in the SQL, so basically the values we fetch at the beginning
                    // after loading the data. But we need a good way to communicate to the UI Code that this should be the full list, vs
                    // wait, we have other constraints, let me fetch them before you display anything.
                    if selection.constraints.len() >= 2 {
                        for constraint in &selection.constraints {
                            // TODO: the database does some more filtering based on the content of the selections. we should only
                            // do this for loop, if the filtered length of the constraints is bigger than two
                            let constraint_towns = self
                                .model
                                .get_towns_for_constraint_with_selection(&constraint, &selection);
                            self.channel_tx
                                .send(MessageToView::TownListConstraint(
                                    constraint.partial_clone(),
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
