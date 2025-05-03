/// I want to know how many users I have and what sort of request they send.
use crate::message::MessageToServer;

static SERVER_POST_LOAD_SERVER: &str = "https://gmap.turun.de/v1/load_server";
static SERVER_POST_STORED_CONFIG: &str = "https://gmap.turun.de/v1/stored_config";

#[cfg(not(target_arch = "wasm32"))]
static SERVER_GET_VERSION: &str = "https://gmap.turun.de/latest_version";

#[cfg(target_arch = "wasm32")]
// the webpage always ships the latest version
pub fn get_latest_version() {}

#[cfg(not(target_arch = "wasm32"))]
/// check on the server what the latest version is.
pub fn get_latest_version() {
    let user_version = env!("CARGO_PKG_VERSION");
    let mut request = ehttp::Request::get(SERVER_GET_VERSION);
    request.body = user_version.to_string().into_bytes();
    ehttp::fetch(request, move |res_respone| {
        if let Err(_err) = res_respone {
            return;
        }
        let response = res_respone.unwrap();

        // equivalent to reqwest !response.status().is_success()
        if !(200..=299).contains(&response.status) {
            return;
        }

        // getting the response text could fail if it's not valid utf-8
        let res_text = response.text();
        if res_text.is_none() {
            return;
        }
        let text = res_text.unwrap();

        let version_and_message: Vec<&str> = text.splitn(2, '\n').collect();
        let (server_version, message) = if version_and_message.is_empty() {
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

        // properly compare versions. Both `user_version` and `server_version` are strings of the form xx.yy.zz
        // parse the strings as numbers, add ".0" at the end of the shorter string if necessary and compare piece by piece
        let user_version_parts = user_version.split('.').collect::<Vec<_>>();
        let server_version_parts = server_version.split('.').collect::<Vec<_>>();
        for index in 0..user_version_parts.len().max(server_version_parts.len()) {
            let user_part = user_version_parts
                .get(index)
                .map(|s| s.parse().unwrap_or(0))
                .unwrap_or(0);
            let server_part = server_version_parts
                .get(index)
                .map(|s| s.parse().unwrap_or(0))
                .unwrap_or(0);

            if user_part < server_part {
                let _result = native_dialog::MessageDialog::new()
                    .set_title(&t!("menu.update_notice.title"))
                    .set_text(&t!(
                        "menu.update_notice.content",
                        user_version = user_version,
                        server_version = server_version,
                        message = message
                    ))
                    .set_type(native_dialog::MessageType::Info)
                    .show_alert();
                return;
            }
        }
    });
}

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
