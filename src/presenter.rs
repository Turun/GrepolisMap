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
        for msg in &self.channel_rx {
            match msg {
                MessageToModel::SetServer(server) => {
                    let db = Database::create_for_world(&server.id).unwrap();
                    let towns = db.get_all_towns();
                    self.model = Model::Loaded { db };
                    self.channel_tx
                        .send(MessageToView::GotServer(towns))
                        .expect("Failed to send message 'got server'");
                }
                MessageToModel::FetchTowns(selection) => {
                    let towns = self.model.get_towns_for_selection(&selection);
                    self.channel_tx
                        .send(MessageToView::TownList(selection, towns))
                        .expect("Failed to send city list to view");
                }
            }
        }
    }
}
