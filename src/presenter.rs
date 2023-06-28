use crate::message::Message;
use crate::model::download::Database;
use crate::model::Model;
use crate::view::View;
use std::sync::mpsc;

pub struct Presenter {
    model: Model,
    view: View,
    channel_tx: mpsc::Sender<Message>,
    channel_rx: mpsc::Receiver<Message>,
}

impl Presenter {
    pub fn new() -> Self {
        let (view_tx, self_rx) = mpsc::channel::<Message>();
        let (self_tx, view_rx) = mpsc::channel::<Message>();

        Self {
            model: Model::Uninitialized,
            view: View::new(view_rx, view_tx),
            channel_tx: self_tx,
            channel_rx: self_rx,
        }
    }

    pub fn start(self) {
        self.view.start();

        for msg in self.channel_rx {
            match msg {
                Message::SetServer(server) => {
                    let db_future = Database::create_for_world(&server.id);
                    let db = futures::executor::block_on(db_future).unwrap();
                    self.model = Model::Loaded { db }
                }
                Message::GotServer => todo!(),
            }
        }
    }
}
