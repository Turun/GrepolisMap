/// I want to know how many users I have and what sort of request they send.
use crate::message::{MessageToServer, MessageToView};

#[cfg(not(target_arch = "wasm32"))]
static SERVER_POST_LOAD_SERVER: &str = "https://gmap.turun.de/v1/load_server";
#[cfg(not(target_arch = "wasm32"))]
static SERVER_POST_STORED_CONFIG: &str = "https://gmap.turun.de/v1/stored_config";
#[cfg(not(target_arch = "wasm32"))]
static SERVER_GET_VERSION: &str = "https://gmap.turun.de/latest_version";

#[cfg(target_arch = "wasm32")]
// on wasm we would have to rewrite to async reqwest clients, just like we had to do in
// download.rs. So I'll just skip it. latest version is useless on the webpage anyway
pub fn get_latest_version() -> Option<MessageToView> {
    None
}

#[cfg(not(target_arch = "wasm32"))]
/// check on the server what the latest version is.
pub fn get_latest_version() -> Option<MessageToView> {
    let user_version = env!("CARGO_PKG_VERSION");
    let client = reqwest::blocking::Client::new();
    let res_response = client.get(SERVER_GET_VERSION).body(user_version).send();
    if let Err(_err) = res_response {
        return None;
    }
    let response = res_response.unwrap();

    if !response.status().is_success() {
        return None;
    }

    let res_text = response.text();
    if let Err(_err) = res_text {
        return None;
    }
    let text = res_text.unwrap();

    let version_and_message: Vec<&str> = text.splitn(2, '\n').collect();
    let (server_version, message) = if version_and_message.len() == 0 {
        return None;
    } else if version_and_message.len() == 1 {
        let version = version_and_message[0];
        (version, "")
    } else if version_and_message.len() == 2 {
        let version = version_and_message[0];
        let message = version_and_message[1];
        (version, message)
    } else {
        return None;
    };

    if user_version >= server_version {
        return None;
    }

    return Some(MessageToView::VersionInfo(
        server_version.to_owned(),
        message.to_owned(),
    ));
}

#[cfg(not(target_arch = "wasm32"))]
pub fn process_messages(messages: &[MessageToServer]) {
    for msg in messages {
        let (url, body) = match msg {
            MessageToServer::LoadServer(server_id) => (SERVER_POST_LOAD_SERVER, server_id),
            MessageToServer::StoredConfig(yaml_string) => (SERVER_POST_STORED_CONFIG, yaml_string),
        };
        ehttp::fetch(
            ehttp::Request::post(url, body.as_bytes().to_vec()),
            |_response| { /*do nothing. I don't think we send anything back anyway*/ },
        );
    }
}

#[cfg(target_arch = "wasm32")]
// on wasm we would have to rewrite to async reqwest clients, just like we had to do in
// download.rs. So I'll just skip it. Load server will have to pass through our server anyway
pub fn process_messages(messages: &[MessageToServer]) {}
