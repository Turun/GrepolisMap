#![warn(clippy::pedantic)]
#![allow(clippy::expect_fun_call)]
// hide the cmd when opening the exe on windows, see: https://github.com/emilk/egui/issues/116
#![windows_subsystem = "windows"]

mod constraint;
mod message;
mod model;
mod presenter;
mod selection;
mod storage;
mod town;
mod view;

use std::{sync::mpsc, thread};

use message::MessageToModel;
use message::MessageToView;
use view::View;

use crate::presenter::Presenter;

static VERSION: &str = "1.2.7";

fn main() {
    let (view_tx, model_rx) = mpsc::channel::<MessageToModel>();
    let (model_tx, view_rx) = mpsc::channel::<MessageToView>();

    let handle = thread::spawn(move || {
        let mut p = Presenter::new(model_rx, model_tx);
        p.start();
    });

    View::new_and_start(view_rx, view_tx);
    handle.join().expect("Failed to join view/presenter handle");
}
