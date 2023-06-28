use crate::message::Message;
use crate::model::download::Database;
use crate::model::Model;
use std::sync::mpsc;

pub struct Presenter {
    model: Model,
    channel_tx: mpsc::Sender<Message>,
    channel_rx: mpsc::Receiver<Message>,
}

impl Presenter {
    pub fn new(rx: mpsc::Receiver<Message>, tx: mpsc::Sender<Message>) -> Self {
        Self {
            model: Model::Uninitialized,
            channel_tx: tx,
            channel_rx: rx,
        }
    }
    pub fn start(&mut self) {
        for msg in &self.channel_rx {
            match msg {
                Message::SetServer(server) => {
                    let db = Database::create_for_world(&server.id).unwrap();
                    self.model = Model::Loaded { db }
                }
                Message::GotServer => todo!(),
            }
        }
    }
}
