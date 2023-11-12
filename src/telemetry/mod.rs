/// I want to know how many users I have and what sort of request they send.
use std::sync::mpsc;

use crate::message::MessageToView;

static SERVER_POST: &str = "https://grepolismap.turun.de/v1/";
static SERVER_VERSION: &str = "https://grepolismap.turun.de/lastest_version";

/// check on the server what the latest version is.
pub fn get_latest_version(view_tx: &mpsc::Sender<MessageToView>) {
    let client = reqwest::blocking::Client::new();
    let res_response = client.get(SERVER_VERSION).send();
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
    let version = version_and_message[0];
    let message = version_and_message[1];

    let _result = view_tx.send(MessageToView::VersionInfo(
        version.to_owned(),
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
