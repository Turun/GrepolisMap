use rusqlite::Row;

/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

#[derive(Debug)]
pub enum MessageToView {
    GotServer(Vec<Town>),
    TownList(TownSelection, Vec<Town>),
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
    pub id: i32,
    pub player_id: Option<i32>,
    pub name: String,
    pub x: i16,
    pub y: i16,
    pub slot_number: u8,
    pub points: u16,
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

#[derive(Debug)]
pub enum TownSelection {
    None,
    All,
    Ghosts,
    Selected(Vec<TownConstraint>),
}

#[derive(Debug)]
pub struct TownConstraint {}
