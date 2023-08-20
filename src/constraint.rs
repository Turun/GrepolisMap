use serde::{Deserialize, Serialize};
use std::default::Default;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use strum_macros::EnumIter;

// TODO: Serialize/Deserialize with custom implementation. We only
// need to save something like "PlayerName == 'erstes'" Instead of
// "{constraint_type: "PlayerName", comparator: "LessThan", vale: "erstes"}"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
    pub comparator: Comparator,
    pub value: String,

    #[serde(skip)] // defaults to None
    pub drop_down_values: Option<Arc<Vec<String>>>,
}

impl Constraint {
    pub fn partial_clone(&self) -> Self {
        Self {
            constraint_type: self.constraint_type,
            comparator: self.comparator,
            value: self.value.clone(),
            drop_down_values: None,
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
            constraint_type: ConstraintType::PlayerName,
            comparator: Comparator::Equal,
            value: String::new(),
            drop_down_values: None,
        }
    }
}

impl Eq for Constraint {}
impl PartialEq for Constraint {
    fn eq(&self, other: &Self) -> bool {
        self.constraint_type == other.constraint_type
            && self.comparator == other.comparator
            && self.value == other.value
    }
}

impl Hash for Constraint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.constraint_type.hash(state);
        self.comparator.hash(state);
        self.value.hash(state);
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    pub fn table(self) -> String {
        match self {
            ConstraintType::PlayerName
            | ConstraintType::PlayerPoints
            | ConstraintType::PlayerRank
            | ConstraintType::PlayerTowns => String::from("players"),
            ConstraintType::AllianceName
            | ConstraintType::AlliancePoints
            | ConstraintType::AllianceTowns
            | ConstraintType::AllianceMembers
            | ConstraintType::AllianceRank => String::from("alliances"),
            ConstraintType::TownName | ConstraintType::TownPoints => String::from("towns"),
            ConstraintType::IslandX
            | ConstraintType::IslandY
            | ConstraintType::IslandTowns
            | ConstraintType::IslandResMore
            | ConstraintType::IslandResLess => String::from("islands"),
        }
    }

    pub fn property(self) -> String {
        match self {
            ConstraintType::PlayerName
            | ConstraintType::AllianceName
            | ConstraintType::TownName => String::from("name"),
            ConstraintType::PlayerPoints
            | ConstraintType::AlliancePoints
            | ConstraintType::TownPoints => String::from("points"),
            ConstraintType::PlayerRank | ConstraintType::AllianceRank => String::from("rank"),
            ConstraintType::PlayerTowns
            | ConstraintType::AllianceTowns
            | ConstraintType::IslandTowns => String::from("towns"),
            ConstraintType::AllianceMembers => String::from("members"),
            ConstraintType::IslandX => String::from("x"),
            ConstraintType::IslandY => String::from("y"),
            ConstraintType::IslandResMore => String::from("ressource_plus"),
            ConstraintType::IslandResLess => String::from("ressource_minus"),
        }
    }

    pub fn is_string(self) -> bool {
        match self {
            ConstraintType::PlayerName
            | ConstraintType::AllianceName
            | ConstraintType::TownName
            | ConstraintType::IslandResMore
            | ConstraintType::IslandResLess => true,

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
            | ConstraintType::IslandTowns => false,
        }
    }
}

#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
