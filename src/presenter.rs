use crate::message::Message;
use crate::model::download::Database;
use crate::model::Model;
use core::panic;
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
                    self.model = Model::Loaded { db };
                    self.channel_tx
                        .send(Message::GotServer)
                        .expect("Failed to send message 'got server'");
                }
                Message::GotServer => {
                    panic!("GotServer should never be sent from the ui to the presenter");
                }
                Message::FetchCities(selection) => {
                    let cities = self.model.get_cities_for_selection(selection);
                    self.channel_tx
                        .send(Message::CityList(cities))
                        .expect("Failed to send city list to view");
                }
            }
        }
    }
}
