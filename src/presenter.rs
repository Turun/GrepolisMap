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
            //TODO better error handling for the database. We should not let the model
            //  thread crash due to DB issues. In
            //  the worst case the user can just try to reload the data
            match message {
                MessageToModel::SetServer(server) => {
                    let db =
                        Database::create_for_world(&server.id, self.channel_tx.clone()).unwrap();
                    let towns = db.get_all_towns();
                    let player_names = db.get_player_names();
                    let alliance_names = db.get_alliance_names(); // TODO split that into extra messages. Send messages back to the Model as early as possible, and in small steps
                    self.model = Model::Loaded { db };
                    self.channel_tx
                        .send(MessageToView::GotServer(
                            towns,
                            player_names,
                            alliance_names,
                        ))
                        .expect("Failed to send message 'got server'");
                }
                MessageToModel::FetchGhosts => {
                    let towns = self.model.get_ghost_towns();
                    self.channel_tx
                        .send(MessageToView::GhostTowns(towns))
                        .expect("Failed to send ghost town list to view");
                }
                MessageToModel::FetchTowns(constraint) => {
                    let towns = self.model.get_towns_for_selection(&constraint);
                    self.channel_tx
                        .send(MessageToView::TownList(constraint, towns))
                        .expect("Failed to send town list to view");
                }
            }
        }
    }
}
