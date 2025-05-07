/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene
/// UPDATE: yeah about that .... lol. Message passing is nice, but it takes some form of good
/// control flow, which I have given up on after switching to a completely sync code model that
/// allows us to run in the browser. Technically I could still make it work, but it is more likely
/// that I will drop messages fully at some point.
use core::fmt;

pub enum PresenterReady {
    AlwaysHasBeen,
    WaitingForAPI,
    NewlyReady,
}

#[allow(clippy::module_name_repetitions)]
pub enum MessageToServer {
    LoadServer(String),
    StoredConfig(String),
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
pub enum MessageToView {
    GotServer,
}

impl fmt::Display for MessageToView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToView::GotServer => {
                write!(f, "MessageToView::GotServer",)
            }
        }
    }
}

#[derive(Debug, Clone)]
// Regarding Progress::BackendCrashed: technically we don't need to remove the displayed
// stuff yet and could the ui state as initialized. The data that is already loaded
// can persist. It's just that the user can't fetch any new data from the backend, so a
// warning about that should be fine.
pub enum Progress {
    None,
    BackendCrashed(String),
    Fetching,
    LoadingFile,
}
