use rusqlite::Row;

/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

#[derive(Debug)]
pub enum MessageToView {
    GotServer,
    TownList(Vec<Town>),
}

#[derive(Debug)]
pub enum MessageToModel {
    SetServer(Server),
    FetchTowns(TownSelection),
}

#[derive(Debug)]
pub struct Server {
    pub id: String,
}

#[derive(Debug)]
pub struct Town {
    id: i32,
    player_id: i32,
    name: String,
    x: i16,
    y: i16,
    slot_number: u8,
    points: u16,
}

impl Town {
    pub fn from(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            player_id: row.get(1)?,
            name: row.get(2)?,
            x: row.get(3)?,
            y: row.get(4)?,
            slot_number: row.get(5)?,
            points: row.get(6)?,
        })
    }
}

pub enum State {
    Uninitialized,
    Show(TownSelection),
}

#[derive(Debug)]
pub enum TownSelection {
    All,
    Ghosts,
    Selected(Vec<TownConstraint>),
}

#[derive(Debug)]
pub struct TownConstraint {}
