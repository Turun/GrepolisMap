use core::fmt;
use rusqlite::Row;
use strum_macros::EnumIter;
use uuid;

/// This is a file for the messages passed between the view and the presenter.
/// message passing communication allows them to be on separate threads. Also it's good code hygene

#[derive(Debug)]
pub enum MessageToView {
    Loading(Progress),
    GotServer,
    AllTowns(Vec<Town>),
    GhostTowns(Vec<Town>),
    DropDownValues(ConstraintType, Vec<String>),
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
            MessageToView::DropDownValues(constraint_type, values) => {
                write!(
                    f,
                    "MessageToView::DropDownValues({}: {} entries)",
                    constraint_type,
                    values.len()
                )
            }
        }
    }
}

pub enum MessageToModel {
    SetServer(Server, egui::Context),
    FetchAll,
    FetchGhosts,
    FetchDropDownValues(ConstraintType),
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
            MessageToModel::FetchDropDownValues(constraint_type) => {
                write!(
                    f,
                    "MessageToModel::FetchDropDownValues({})",
                    constraint_type
                )
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
        Ok(Self {
            id: row.get(0)?,
            player_id: row.get(1)?,
            player_name: row.get(9)?,
            alliance_name: row.get(10)?,
            name: row.get(2)?,
            x: row.get::<usize, f32>(3)? + row.get::<usize, f32>(7)? / 125.0,
            y: row.get::<usize, f32>(4)? + row.get::<usize, f32>(8)? / 125.0,
            slot_number: row.get(5)?,
            points: row.get(6)?,
        })
    }
}

pub enum Change {
    MoveUp(usize),
    Remove(usize),
    MoveDown(usize),
}

#[derive(Debug, Clone)]
pub struct TownConstraint {
    uuid: uuid::Uuid,
    pub state: ConstraintState,
    pub constraints: Vec<Constraint>,
    pub color: egui::Color32,
    pub towns: Vec<Town>,
}

#[derive(Debug, Clone)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
    pub comparator: Comparator,
    pub value: String,
}

impl Default for Constraint {
    fn default() -> Self {
        Self {
            constraint_type: ConstraintType::PlayerName,
            comparator: Comparator::Equal,
            value: String::from(""),
        }
    }
}

#[derive(Debug, Clone, EnumIter, PartialEq, Eq, Hash)]
pub enum ConstraintType {
    PlayerName,
    PlayerPoints,
    PlayerRank,
    PlayerTowns,
    AllianceName,
    AlliancePoints,
    AllianceTowns,
    AllianceMembers,
    AllianceRank,
    TownName,
    TownPoints,
    IslandX,
    IslandY,
    IslandTowns,
    IslandResMore,
    IslandResLess,
}

impl fmt::Display for ConstraintType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintType::PlayerName => write!(f, "PlayerName"),
            ConstraintType::PlayerPoints => write!(f, "PlayerPoints"),
            ConstraintType::PlayerRank => write!(f, "PlayerRank"),
            ConstraintType::PlayerTowns => write!(f, "PlayerTowns"),
            ConstraintType::AllianceName => write!(f, "AllianceName"),
            ConstraintType::AlliancePoints => write!(f, "AlliancePoints"),
            ConstraintType::AllianceTowns => write!(f, "AllianceTowns"),
            ConstraintType::AllianceMembers => write!(f, "AllianceMembers"),
            ConstraintType::AllianceRank => write!(f, "AllianceRank"),
            ConstraintType::TownName => write!(f, "TownName"),
            ConstraintType::TownPoints => write!(f, "TownPoints"),
            ConstraintType::IslandX => write!(f, "IslandX"),
            ConstraintType::IslandY => write!(f, "IslandY"),
            ConstraintType::IslandTowns => write!(f, "IslandTowns"),
            ConstraintType::IslandResMore => write!(f, "IslandResMore"),
            ConstraintType::IslandResLess => write!(f, "IslandResLess"),
        }
    }
}

impl ConstraintType {
    pub fn table(&self) -> String {
        match self {
            ConstraintType::PlayerName
            | ConstraintType::PlayerPoints
            | ConstraintType::PlayerRank
            | ConstraintType::PlayerTowns => return String::from("players"),
            ConstraintType::AllianceName
            | ConstraintType::AlliancePoints
            | ConstraintType::AllianceTowns
            | ConstraintType::AllianceMembers
            | ConstraintType::AllianceRank => return String::from("alliances"),
            ConstraintType::TownName | ConstraintType::TownPoints => return String::from("towns"),
            ConstraintType::IslandX
            | ConstraintType::IslandY
            | ConstraintType::IslandTowns
            | ConstraintType::IslandResMore
            | ConstraintType::IslandResLess => return String::from("islands"),
        }
    }

    pub fn property(&self) -> String {
        match self {
            ConstraintType::PlayerName
            | ConstraintType::AllianceName
            | ConstraintType::TownName => return String::from("name"),
            ConstraintType::PlayerPoints
            | ConstraintType::AlliancePoints
            | ConstraintType::TownPoints => return String::from("points"),
            ConstraintType::PlayerRank | ConstraintType::AllianceRank => {
                return String::from("rank")
            }
            ConstraintType::PlayerTowns
            | ConstraintType::AllianceTowns
            | ConstraintType::IslandTowns => return String::from("towns"),
            ConstraintType::AllianceMembers => return String::from("members"),
            ConstraintType::IslandX => return String::from("x"),
            ConstraintType::IslandY => return String::from("y"),
            ConstraintType::IslandResMore => return String::from("ressource_plus"),
            ConstraintType::IslandResLess => return String::from("ressource_minus"),
        }
    }

    pub fn is_string(&self) -> bool {
        match self {
            ConstraintType::PlayerName
            | ConstraintType::AllianceName
            | ConstraintType::TownName
            | ConstraintType::IslandResMore
            | ConstraintType::IslandResLess => return true,

            ConstraintType::PlayerPoints
            | ConstraintType::PlayerRank
            | ConstraintType::PlayerTowns
            | ConstraintType::AlliancePoints
            | ConstraintType::AllianceTowns
            | ConstraintType::AllianceMembers
            | ConstraintType::AllianceRank
            | ConstraintType::TownPoints
            | ConstraintType::IslandX
            | ConstraintType::IslandY
            | ConstraintType::IslandTowns => return false,
        }
    }
}

#[derive(Debug, Clone, EnumIter, PartialEq, Eq)]
pub enum Comparator {
    LessThan,
    Equal,
    GreaterThan,
    NotEqual,
}

impl fmt::Display for Comparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Comparator::LessThan => write!(f, "<="),
            Comparator::Equal => write!(f, "="),
            Comparator::GreaterThan => write!(f, ">="),
            Comparator::NotEqual => write!(f, "<>"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ConstraintState {
    Loading,
    Finished,
}

impl TownConstraint {
    pub fn new() -> Self {
        Self {
            uuid: uuid::Uuid::new_v4(),
            state: ConstraintState::Finished,
            towns: Vec::new(),
            constraints: Vec::new(),
            color: egui::Color32::GREEN,
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
            "TownConstraint({} constraints, {} towns)",
            self.constraints.len(),
            self.towns.len()
        )
    }
}
