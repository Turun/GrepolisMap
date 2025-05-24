#![warn(clippy::pedantic)]
#![allow(clippy::to_string_trait_impl)]
#![allow(clippy::needless_return)]
// hide the cmd when opening the exe on windows, see: https://github.com/emilk/egui/issues/116
#![windows_subsystem = "windows"]

#[macro_use]
extern crate rust_i18n;
i18n!("locales", fallback = "en");

mod constraint;
mod emptyconstraint;
mod emptyselection;
mod model;
mod presenter;
mod selection;
mod telemetry;
mod town;
mod view;

#[cfg(not(target_arch = "wasm32"))]
mod storage;
#[cfg(target_arch = "wasm32")]
mod wasm_utils;

use view::View;

// before we had the default from eframe, "app"
// const APP_KEY: &str = "TESTING";
const APP_KEY: &str = eframe::APP_KEY;

fn main() {
    View::new_and_start();
}
