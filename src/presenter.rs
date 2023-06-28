use crate::message::{MessageToModel, MessageToView};
use crate::model::download::Database;
use crate::model::Model;
use std::sync::mpsc;

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
                MessageToModel::FetchTowns(constraint) => {
                    let towns = self.model.get_towns_for_selection(&constraint);
                    self.channel_tx
                        .send(MessageToView::TownList(constraint, towns))
                        .expect("Failed to send town list to view");
                }
            }

            // after we process a message, tell the UI to repaint
            self.model.request_repaint();
        }
    }
}
