mod message;
mod model;
mod presenter;
mod view;

use std::{sync::mpsc, thread};

use message::MessageToModel;
use message::MessageToView;
use view::View;

use crate::presenter::Presenter;

fn main() {
    let (view_tx, model_rx) = mpsc::channel::<MessageToModel>();
    let (model_tx, view_rx) = mpsc::channel::<MessageToView>();

    let view = View::new(view_rx, view_tx);

    let handle = thread::spawn(move || {
        let mut p = Presenter::new(model_rx, model_tx);
        p.start();
    });

    view.start();
    handle.join().expect("Failed to join view handle");
}
