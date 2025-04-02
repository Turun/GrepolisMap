#![warn(clippy::pedantic)]
#![allow(clippy::expect_fun_call)]
// hide the cmd when opening the exe on windows, see: https://github.com/emilk/egui/issues/116
#![windows_subsystem = "windows"]

#[macro_use]
extern crate rust_i18n;
i18n!("locales", fallback = "en");

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

use view::View;

// before we had the default from eframe, "app"
// const APP_KEY: &str = "TESTING";
const APP_KEY: &str = eframe::APP_KEY;

fn main() {
    View::new_and_start();
}
