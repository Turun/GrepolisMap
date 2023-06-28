use crate::message::TownSelection;

pub enum State {
    Uninitialized,
    Show(TownSelection),
}
