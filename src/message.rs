use core::fmt;

use rusqlite::Row;

/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

#[derive(Debug)]
pub enum MessageToView {
    Loading(Progress),
    GotServer(Vec<Town>),
    TownList(TownSelection, Vec<Town>),
}

impl fmt::Display for MessageToView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToView::GotServer(all_towns) => {
                write!(f, "MessageToView::GotServer({} towns)", all_towns.len())
            }
            MessageToView::TownList(selection, towns) => write!(
                f,
                "MessageToView::TownList({:?}, {} towns)",
                selection,
                towns.len()
            ),
            MessageToView::Loading(progress) => write!(f, "MessageToView::Loading({:?})", progress),
        }
    }
}

#[derive(Debug)]
pub enum MessageToModel {
    SetServer(Server),
    FetchTowns(TownSelection),
}

impl fmt::Display for MessageToModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToModel::SetServer(server) => {
                write!(f, "MessageToMode::SetServer({})", server.id)
            }
            MessageToModel::FetchTowns(selection) => {
                write!(f, "MessageToModel::FetchTowns({:?})", selection)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Progress {
    None,
    IslandOffsets,
    Alliances,
    Players,
    Towns,
    Islands,
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
    pub x: f32,
    pub y: f32,
    pub slot_number: u8,
    pub points: u16,
}

impl Town {
    pub fn from(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Self {
            id: row.get(0)?,
            player_id: row.get(1)?,
            name: row.get(2)?,
            x: row.get::<usize, f32>(3)? + row.get::<usize, f32>(7)? / 125.0,
            y: row.get::<usize, f32>(4)? + row.get::<usize, f32>(8)? / 125.0,
            slot_number: row.get(5)?,
            points: row.get(6)?,
        })
    }
}

#[derive(Debug, Clone)]
pub enum TownSelection {
    None,
    All,
    Ghosts,
    Selected(Vec<TownConstraint>),
}

#[derive(Debug, Clone)]
pub struct TownConstraint {}
