mod message;
mod model;
mod presenter;
mod view;

use crate::presenter::Presenter;

fn main() {
    let p = Presenter::new();
    p.start()
}
