use crate::message::Progress;

#[derive(Debug, Clone)]
pub enum State {
    Uninitialized(Progress),
    Show,
}
