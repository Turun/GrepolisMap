use base64::Engine;
use log::debug;
use wasm_bindgen::JsCast;
use wasm_bindgen::{closure::Closure, JsValue};
/// some utility functions that are required for some features on wasm
use web_sys;

use crate::emptyselection::EmptyTownSelection;
use crate::selection::TownSelection;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};

fn get_current_url_search_param_string() -> Option<String> {
    web_sys::window()
        .map(|w| w.location().search().ok())
        .flatten()
}

/// take the current url, which is something like gmap.turun.de/?server=de99&selections=base64encodedSelections
/// and return a tuple of the server id and the list of selections
pub fn parse_current_url() -> (
    Option<String>,
    Option<String>,
    Option<Vec<EmptyTownSelection>>,
) {
    let search_param_string_opt = web_sys::window()
        .map(|w| w.location().search().ok())
        .flatten();

    if search_param_string_opt.is_none() {
        return (None, None, None);
    }
    let search_param_string = search_param_string_opt.clone().unwrap();
    let url_params_opt = web_sys::UrlSearchParams::new_with_str(&search_param_string).ok();

    if url_params_opt.is_none() {
        return (search_param_string_opt, None, None);
    }
    let url_params = url_params_opt.unwrap();

    let server_id = url_params.get("server");
    let selections = url_params
        .get("selections")
        .map(|b64str| URL_SAFE.decode(b64str).ok())
        .flatten()
        .map(|bytes| String::from_utf8(bytes).ok())
        .flatten()
        .map(|selections_str| EmptyTownSelection::try_from_str(&selections_str).ok())
        .flatten();

    return (search_param_string_opt, server_id, selections);
}

/// Convert the given arguments into a urlencoded string. e.g. /?server=de99&selections=base64encodedSelections
/// and then set the current window url to that.
pub fn set_current_url(text: &str) {
    let window = web_sys::window().expect("no global `window`");
    let history = window.history().expect("should have history");
    // args: state object, title (ignored), url (must be same-origin)
    let _res = history.replace_state_with_url(&JsValue::NULL, "", Some(text));
    // res.expect("failed to update history state");
}

/// Convert the given arguments into a urlencoded string. e.g. /?server=de99&selections=base64encodedSelections
pub fn state_to_url_string(
    server_id: Option<&str>,
    selections: Option<&[TownSelection]>,
) -> String {
    let mut url_text = String::from("?");
    let mut url_parts = Vec::new();
    if let Some(server_id) = server_id {
        url_parts.push(format!("server={server_id}"));
    }
    if let Some(selections) = selections {
        if !selections.is_empty() {
            let text = serde_yaml::to_string(selections).unwrap();
            let text = URL_SAFE.encode(text);
            url_parts.push(format!("selections={text}"));
        }
    }
    url_text.push_str(&url_parts.join("&"));
    return url_text;
}

// fn register_popstate_handler<F>(mut on_pop: F)
// where
//     F: 'static + FnMut(String),
// {
//     // TODO: this is a template from ChatGPT that we still need to modify if we want to properly handle forwards/backwards naviation in the browser

//     let window: Window = web_sys::window().expect("no global `window`");
//     // Wrap your callback in a `Closure` so it can be passed to JS
//     let cb = Closure::wrap(Box::new(move |e: PopStateEvent| {
//         // Read the new URL from window.location
//         let url = window.location().href().unwrap_or_default();
//         on_pop(url);
//     }) as Box<dyn FnMut(_)>);

//     window
//         .add_event_listener_with_callback("popstate", cb.as_ref().unchecked_ref())
//         .unwrap();

//     // Prevent the closure from being freed, so it lives for the full session
//     cb.forget();
// }
