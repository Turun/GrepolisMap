mod message;
mod model;
mod presenter;
mod view;

use std::{sync::mpsc, thread};

use message::Message;
use view::View;

use crate::presenter::Presenter;

fn main() {
    let (view_tx, self_rx) = mpsc::channel::<Message>();
    let (self_tx, view_rx) = mpsc::channel::<Message>();

    let view = View::new(view_rx, view_tx);

    let mut p = Presenter::new(self_rx, self_tx);
    let handle = thread::spawn(move || {
        p.start();
    });

    view.start();
    handle.join().expect("Failed to join view handle");
}
