use rusqlite::Row;
use std::default::Default;
use std::fmt;
use strum_macros::EnumIter;
use uuid;

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
    Add,
    MoveUp(usize),
    Remove(usize),
    MoveDown(usize),
}
#[derive(Debug, Clone)]
pub struct Constraint {
    uuid: uuid::Uuid,
    pub constraint_type: ConstraintType,
    pub comparator: Comparator,
    pub value: String,
    pub drop_down_values: Vec<String>,
}

impl Constraint {
    pub fn partial_clone(&self) -> Self {
        Self {
            uuid: self.uuid.clone(),
            constraint_type: self.constraint_type.clone(),
            comparator: self.comparator.clone(),
            value: self.value.clone(),
            drop_down_values: Vec::new(),
        }
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Constraint({} {} {})",
            self.constraint_type, self.comparator, self.value
        )
    }
}

impl Default for Constraint {
    fn default() -> Self {
        Self {
            uuid: uuid::Uuid::new_v4(),
            constraint_type: ConstraintType::PlayerName,
            comparator: Comparator::Equal,
            value: String::from(""),
            drop_down_values: Vec::new(),
        }
    }
}

impl Eq for Constraint {}
impl PartialEq for Constraint {
    fn eq(&self, other: &Self) -> bool {
        self.uuid == other.uuid
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
pub enum SelectionState {
    Loading,
    Finished,
}

#[derive(Debug, Clone)]
pub struct TownSelection {
    uuid: uuid::Uuid,
    pub state: SelectionState,
    pub constraints: Vec<Constraint>,
    pub color: egui::Color32,
    pub towns: Vec<Town>,
}

impl TownSelection {
    /// Clone the TownSelection, but without the list of towns. Less memory
    /// required and we can reconstruct the list of towns anyway, if given
    /// the list of constraints.
    pub fn partial_clone(&self) -> Self {
        Self {
            towns: Vec::new(),
            uuid: self.uuid.clone(),
            state: self.state.clone(),
            constraints: self.constraints.clone(),
            color: self.color.clone(),
        }
    }
}

impl Default for TownSelection {
    fn default() -> Self {
        Self {
            uuid: uuid::Uuid::new_v4(),
            state: SelectionState::Finished,
            towns: Vec::new(),
            constraints: vec![Constraint::default()],
            color: egui::Color32::GREEN,
        }
    }
}

impl PartialEq<TownSelection> for &mut TownSelection {
    fn eq(&self, other: &TownSelection) -> bool {
        self.uuid == other.uuid
    }
}

impl fmt::Display for TownSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "TownSelection({} constraints, {} towns)",
            self.constraints.len(),
            self.towns.len()
        )
    }
}
