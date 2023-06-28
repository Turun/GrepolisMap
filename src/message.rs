use core::fmt;

use rusqlite::Row;

/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

#[derive(Debug)]
pub enum MessageToView {
    Loading(Progress),
    GhostTowns(Vec<Town>),
    GotServer(Vec<Town>, Vec<String>, Vec<String>),
    TownList(TownConstraint, Vec<Town>),
}

impl fmt::Display for MessageToView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToView::GotServer(all_towns, players, alliances) => {
                write!(
                    f,
                    "MessageToView::GotServer({} towns, {} players, {} alliances)",
                    all_towns.len(),
                    players.len(),
                    alliances.len()
                )
            }
            MessageToView::TownList(selection, towns) => write!(
                f,
                "MessageToView::TownList({:?}, {} towns)",
                selection,
                towns.len()
            ),
            MessageToView::Loading(progress) => write!(f, "MessageToView::Loading({:?})", progress),
            MessageToView::GhostTowns(towns) => {
                write!(f, "MessageToView::GhostTowns({} towns)", towns.len())
            }
        }
    }
}

#[derive(Debug)]
pub enum MessageToModel {
    SetServer(Server),
    FetchGhosts,
    FetchTowns(TownConstraint),
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
            MessageToModel::FetchGhosts => {
                write!(f, "MessageToModel::FetchGhosts")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum Progress {
    None,
    Started,
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

#[derive(Debug, Clone)]
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
        let name = form_urlencoded::parse(row.get::<usize, String>(2)?.as_bytes())
            .map(|(key, val)| [key, val].concat())
            .collect::<String>();
        Ok(Self {
            id: row.get(0)?,
            player_id: row.get(1)?,
            name,
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
    Selected(TownConstraint),
}

#[derive(Debug, Clone)]
pub struct TownConstraint {
    pub from_type: FromType,
    pub color: egui::Color32,
    pub value: String,
    pub towns: Vec<Town>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FromType {
    Player,
    Alliance,
}
