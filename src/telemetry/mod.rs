/// I want to know how many users I have and what sort of request they send.
use crate::view::preferences::{self, Telemetry};

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
                .map_or(0, |s| s.parse().unwrap_or(0));
            let server_part = server_version_parts
                .get(index)
                .map_or(0, |s| s.parse().unwrap_or(0)); // the first 0 is for when there is not part left (turn 2.1 into 2.1.0). The second 0 is for when the number fails to parse (turn 1.two.3 into 1.0.3)

            // I just like <, ==, > more than match x.cmp(&y) {Ordering::Greater => {...}}, so:
            #[allow(clippy::comparison_chain)]
            if user_part < server_part {
                // if the user version is smaller than the server version, show a message dialog that there is a newer one available
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
            } else if user_part == server_part {
                // if the xth digit (and all preceeding ones) are identical, check the (x+1)th digit
                continue;
            } else if user_part > server_part {
                // somehow the user version is bigger than the server version. I don't know how that happened, but we'll just ignore it and return.
                return;
            }
        }
    });
}

/// if allowed by `telemetry_preferences`, we will tell the server that the user has just loaded a new map.
pub fn event_load_server(telemetry_preferences: preferences::Telemetry, server_id: &str) {
    match telemetry_preferences {
        Telemetry::All => {
            ehttp::fetch(
                ehttp::Request::post(SERVER_POST_LOAD_SERVER, server_id.as_bytes().to_vec()),
                |_response| { /*do nothing. I don't think we send anything back anyway*/ },
            );
        }
        Telemetry::OnlyVersionCheck | Telemetry::Nothing => {}
    }
}

/// if allowed by `telemetry_preferences` we will tell the server what selections the user had build the last time they used the program.
pub fn event_stored_config(telemetry_preferences: preferences::Telemetry, yaml_string: &str) {
    match telemetry_preferences {
        Telemetry::All => {
            ehttp::fetch(
                ehttp::Request::post(SERVER_POST_STORED_CONFIG, yaml_string.as_bytes().to_vec()),
                |_response| { /*do nothing. I don't think we send anything back anyway*/ },
            );
        }
        Telemetry::OnlyVersionCheck | Telemetry::Nothing => {}
    }
}
