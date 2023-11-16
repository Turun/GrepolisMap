/// I want to know how many users I have and what sort of request they send.
use std::sync::mpsc;

use crate::message::MessageToView;

static SERVER_POST: &str = "https://grepolismap.turun.de/v1/";
static SERVER_VERSION: &str = "https://grepolismap.turun.de/lastest_version?installed=";

/// check on the server what the latest version is.
pub fn get_latest_version(view_tx: &mpsc::Sender<MessageToView>) {
    let user_version = env!("CARGO_PKG_VERSION");
    let client = reqwest::blocking::Client::new();
    let res_response = client.get(SERVER_VERSION.to_owned() + user_version).send();
    if let Err(_err) = res_response {
        return;
    }
    let response = res_response.unwrap();

    let res_text = response.text();
    if let Err(_err) = res_text {
        return;
    }
    let text = res_text.unwrap();

    let version_and_message: Vec<&str> = text.splitn(2, '\n').collect();
    let (server_version, message) = if version_and_message.len() == 0 {
        return;
    } else if version_and_message.len() == 1 {
        let version = version_and_message[0];
        (version, "")
    } else if version_and_message.len() == 2 {
        let version = version_and_message[0];
        let message = version_and_message[1];
        (version, message)
    } else {
        return;
    };

    if user_version >= server_version {
        return;
    }

    let _result = view_tx.send(MessageToView::VersionInfo(
        server_version.to_owned(),
        message.to_owned(),
    ));
}

pub fn channel_processor(rx: mpsc::Receiver<(String, String)>) {
    let client = reqwest::blocking::Client::new();
    for (endpoint, message) in rx {
        let _result = client
            .post(SERVER_POST.to_owned() + &endpoint)
            .body(message)
            .send();
    }
}
