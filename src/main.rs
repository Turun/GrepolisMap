#![warn(clippy::pedantic)]
#![allow(clippy::expect_fun_call)]
// hide the cmd when opening the exe on windows, see: https://github.com/emilk/egui/issues/116
#![windows_subsystem = "windows"]

mod constraint;
mod emptyconstraint;
mod emptyselection;
mod message;
mod model;
mod presenter;
mod selection;
mod storage;
mod telemetry;
mod town;
mod view;

use std::{sync::mpsc, thread};

use message::MessageToModel;
use message::MessageToView;
use view::View;

use crate::presenter::Presenter;

fn main() {
    let (tx_to_model, model_rx) = mpsc::channel::<MessageToModel>();
    let (tx_to_view, view_rx) = mpsc::channel::<MessageToView>();
    let (tx_to_telemetry, telemetry_rx) = mpsc::channel::<(String, String)>();

    // use std::time::Duration;
    // let test_tx_to_view = tx_to_view.clone();
    // let _handle_test = thread::spawn(move || {
    //     thread::sleep(Duration::from_secs(5));
    //     let _result = test_tx_to_view.send(MessageToView::VersionInfo(
    //         "1.2.9".to_owned(),
    //         "This is a test message".to_owned(),
    //     ));
    // });

    let presenter_tx_to_view = tx_to_view.clone();
    let presenter_tx_to_telemetry = tx_to_telemetry.clone();
    let handle_presenter = thread::spawn(move || {
        let mut p = Presenter::new(model_rx, presenter_tx_to_view, presenter_tx_to_telemetry);
        p.start();
    });

    let telemetry_tx_to_view = tx_to_view;
    let handle_telemetry = thread::spawn(move || {
        telemetry::get_latest_version(&telemetry_tx_to_view);
        telemetry::channel_processor(telemetry_rx);
    });

    View::new_and_start(view_rx, tx_to_model, tx_to_telemetry);
    handle_presenter
        .join()
        .expect("Failed to join view/presenter handle");
    handle_telemetry
        .join()
        .expect("Failed to join view/telemetry handle");
}
