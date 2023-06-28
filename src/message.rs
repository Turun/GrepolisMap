use core::fmt;
use uuid;

use rusqlite::Row;

/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

#[derive(Debug)]
pub enum MessageToView {
    Loading(Progress),
    GotServer,
    AllTowns(Vec<Town>),
    GhostTowns(Vec<Town>),
    PlayerNames(Vec<String>),
    AllianceNames(Vec<String>),
    TownList(TownConstraint, Vec<Town>),
}

impl fmt::Display for MessageToView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToView::GotServer => {
                write!(f, "MessageToView::GotServer",)
            }
            MessageToView::TownList(selection, towns) => write!(
                f,
                "MessageToView::TownList({}, {} towns)",
                selection,
                towns.len()
            ),
            MessageToView::Loading(progress) => write!(f, "MessageToView::Loading({:?})", progress),
            MessageToView::AllTowns(towns) => {
                write!(f, "MessageToView::AllTowns({} towns)", towns.len())
            }
            MessageToView::GhostTowns(towns) => {
                write!(f, "MessageToView::GhostTowns({} towns)", towns.len())
            }
            MessageToView::PlayerNames(names) => {
                write!(f, "MessageToView::PlayerNames({} players)", names.len())
            }
            MessageToView::AllianceNames(names) => {
                write!(f, "MessageToView::AllianceNames({} alliances)", names.len())
            }
        }
    }
}

pub enum MessageToModel {
    SetServer(Server, egui::Context),
    FetchAll,
    FetchGhosts,
    FetchPlayers,
    FetchAlliances,
    FetchTowns(TownConstraint),
}

impl fmt::Display for MessageToModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageToModel::SetServer(server, _frame) => {
                write!(f, "MessageToMode::SetServer({})", server.id)
            }
            MessageToModel::FetchTowns(selection) => {
                write!(f, "MessageToModel::FetchTowns({})", selection)
            }
            MessageToModel::FetchAll => {
                write!(f, "MessageToModel::FetchAll")
            }
            MessageToModel::FetchGhosts => {
                write!(f, "MessageToModel::FetchGhosts")
            }
            MessageToModel::FetchPlayers => {
                write!(f, "MessageToModel::FetchPlayers")
            }
            MessageToModel::FetchAlliances => {
                write!(f, "MessageToModel::FetchAlliances")
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
    pub player_name: Option<String>,
    pub alliance_name: Option<String>,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub slot_number: u8,
    pub points: u16,
}

impl Town {
    pub fn from(row: &Row) -> Result<Self, rusqlite::Error> {
        let town_name = form_urlencoded::parse(row.get::<usize, String>(2)?.as_bytes())
            .map(|(key, val)| [key, val].concat())
            .collect::<String>();
        Ok(Self {
            id: row.get(0)?,
            player_id: row.get(1)?,
            player_name: row.get(9)?,
            alliance_name: row.get(10)?,
            name: town_name,
            x: row.get::<usize, f32>(3)? + row.get::<usize, f32>(7)? / 125.0,
            y: row.get::<usize, f32>(4)? + row.get::<usize, f32>(8)? / 125.0,
            slot_number: row.get(5)?,
            points: row.get(6)?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct TownConstraint {
    // TODO more flexible constraints: e.g. any player not in Alliance X, Y, or Z, with Player Points between 123 and 345
    uuid: uuid::Uuid,
    pub state: ConstraintState,
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConstraintState {
    Loading,
    Finished,
}

impl TownConstraint {
    pub fn new(from_type: FromType, color: egui::Color32, value: String) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4(),
            state: ConstraintState::Finished,
            from_type,
            color,
            value,
            towns: Vec::new(),
        }
    }
}

impl PartialEq<TownConstraint> for &mut TownConstraint {
    fn eq(&self, other: &TownConstraint) -> bool {
        self.uuid == other.uuid
    }
}

impl fmt::Display for TownConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TownConstraint({}, {}, {} towns)",
            match self.from_type {
                FromType::Player => "Player",
                FromType::Alliance => "Alliance",
            },
            self.value,
            self.towns.len()
        )
    }
}
