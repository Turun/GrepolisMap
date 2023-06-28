/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

#[derive(Debug)]
pub enum Message {
    /// tell the backend the user entered a server
    SetServer(Server),

    /// the backend fetched all data for the given server
    GotServer,
}

#[derive(Debug)]
pub struct Server {
    pub id: String,
}
