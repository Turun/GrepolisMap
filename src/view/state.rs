use crate::message::{Progress, TownSelection};

#[derive(Debug, Clone)]
pub enum State {
    Uninitialized(Progress),
    Show(TownSelection),
}
