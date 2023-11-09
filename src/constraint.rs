use crate::emptyconstraint::EmptyConstraint;
use crate::view::dropdownbox::DropDownBox;
use crate::view::Change;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::Hash;
use std::sync::Arc;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Clone)]
pub struct Constraint {
    pub constraint_type: ConstraintType,
    pub comparator: Comparator,
    pub value: String,
    pub drop_down_values: Option<Arc<Vec<String>>>,
}

impl Default for Constraint {
    fn default() -> Self {
        EmptyConstraint::default().fill()
    }
}

impl PartialEq<EmptyConstraint> for Constraint {
    fn eq(&self, other: &EmptyConstraint) -> bool {
        self.constraint_type == other.constraint_type
            && self.comparator == other.comparator
            && self.value == other.value
    }
}

impl fmt::Debug for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Constraint({} {} {}, {} ddv)",
            self.constraint_type,
            self.comparator,
            self.value,
            self.drop_down_values.as_ref().map_or(0, |x| x.len())
        )
    }
}

impl Constraint {
    pub fn partial_clone(&self) -> EmptyConstraint {
        EmptyConstraint {
            constraint_type: self.constraint_type,
            comparator: self.comparator,
            value: self.value.clone(),
        }
    }

    pub fn make_ui(
        &mut self,
        ui: &mut egui::Ui,
        selection_index: usize,
        constraint_index: usize,
        last_item: bool,
    ) -> (Option<Change>, bool) {
        let mut re_edited = false;
        let mut re_change = None;

        ui.horizontal(|ui| {
            // Filter for which attribute?
            let _inner_response = egui::ComboBox::from_id_source(format!(
                "ComboxBox {selection_index}/{constraint_index} Type"
            ))
            .width(ui.style().spacing.interact_size.x * 3.5)
            .selected_text(format!("{}", self.constraint_type))
            .show_ui(ui, |ui| {
                for value in ConstraintType::iter() {
                    let text = value.to_string();
                    if ui
                        .selectable_value(&mut self.constraint_type, value, text)
                        .clicked()
                    {
                        re_edited = true;
                    }
                }
            });

            // with which comparison method (<=, ==, >=, <>)?
            let _inner_response = egui::ComboBox::from_id_source(format!(
                "ComboxBox {selection_index}/{constraint_index} Comparator"
            ))
            .width(ui.style().spacing.interact_size.x * 1.75)
            .selected_text(format!("{}", self.comparator))
            .show_ui(ui, |ui| {
                for value in Comparator::iter() {
                    let text = value.to_string();
                    if ui
                        .selectable_value(&mut self.comparator, value, text)
                        .clicked()
                    {
                        re_edited = true;
                    }
                }
            });

            // List of possible values
            let ddb = DropDownBox::from_iter(
                self.drop_down_values.as_ref(),
                format!("ComboBox {selection_index}/{constraint_index} Value"),
                &mut self.value,
            );
            if ui
                .add_sized(
                    [
                        ui.style().spacing.interact_size.x * 4.5,
                        ui.style().spacing.interact_size.y,
                    ],
                    ddb,
                )
                .changed()
            {
                re_edited = true;
            };

            // Buttons
            if last_item {
                if ui.button("+").clicked() {
                    re_change = Some(Change::Add);
                }
            } else {
                ui.label("and");
            }
            if ui.button("-").clicked() {
                re_change = Some(Change::Remove(constraint_index));
            }
            if ui.button("↑").clicked() {
                re_change = Some(Change::MoveUp(constraint_index));
            }
            if ui.button("↓").clicked() {
                re_change = Some(Change::MoveDown(constraint_index));
            }
        });

        (re_change, re_edited)
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConstraintType {
    PlayerID,
    PlayerName,
    PlayerPoints,
    PlayerRank,
    PlayerTowns,
    AllianceName,
    AlliancePoints,
    AllianceTowns,
    AllianceMembers,
    AllianceRank,
    TownID,
    TownName,
    TownPoints,
    IslandID,
    IslandX,
    IslandY,
    IslandType,
    IslandTowns,
    IslandResMore,
    IslandResLess,
}

impl fmt::Display for ConstraintType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstraintType::PlayerID => write!(f, "PlayerID"),
            ConstraintType::PlayerName => write!(f, "PlayerName"),
            ConstraintType::PlayerPoints => write!(f, "PlayerPoints"),
            ConstraintType::PlayerRank => write!(f, "PlayerRank"),
            ConstraintType::PlayerTowns => write!(f, "PlayerTowns"),
            ConstraintType::AllianceName => write!(f, "AllianceName"),
            ConstraintType::AlliancePoints => write!(f, "AlliancePoints"),
            ConstraintType::AllianceTowns => write!(f, "AllianceTowns"),
            ConstraintType::AllianceMembers => write!(f, "AllianceMembers"),
            ConstraintType::AllianceRank => write!(f, "AllianceRank"),
            ConstraintType::TownID => write!(f, "TownID"),
            ConstraintType::TownName => write!(f, "TownName"),
            ConstraintType::TownPoints => write!(f, "TownPoints"),
            ConstraintType::IslandID => write!(f, "IslandID"),
            ConstraintType::IslandX => write!(f, "IslandX"),
            ConstraintType::IslandY => write!(f, "IslandY"),
            ConstraintType::IslandType => write!(f, "IslandType"),
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
            | ConstraintType::PlayerID
            | ConstraintType::PlayerPoints
            | ConstraintType::PlayerRank
            | ConstraintType::PlayerTowns => String::from("players"),
            ConstraintType::AllianceName
            | ConstraintType::AlliancePoints
            | ConstraintType::AllianceTowns
            | ConstraintType::AllianceMembers
            | ConstraintType::AllianceRank => String::from("alliances"),
            ConstraintType::TownName | ConstraintType::TownPoints | ConstraintType::TownID => {
                String::from("towns")
            }
            ConstraintType::IslandID
            | ConstraintType::IslandX
            | ConstraintType::IslandY
            | ConstraintType::IslandType
            | ConstraintType::IslandTowns
            | ConstraintType::IslandResMore
            | ConstraintType::IslandResLess => String::from("islands"),
        }
    }

    pub fn property(self) -> String {
        match self {
            ConstraintType::PlayerID => String::from("player_id"),
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
            ConstraintType::TownID => String::from("town_id"),
            ConstraintType::AllianceMembers => String::from("members"),
            ConstraintType::IslandID => String::from("island_id"),
            ConstraintType::IslandX => String::from("x"),
            ConstraintType::IslandY => String::from("y"),
            ConstraintType::IslandType => String::from("type"),
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

            ConstraintType::PlayerID
            | ConstraintType::PlayerPoints
            | ConstraintType::PlayerRank
            | ConstraintType::PlayerTowns
            | ConstraintType::AlliancePoints
            | ConstraintType::AllianceTowns
            | ConstraintType::AllianceMembers
            | ConstraintType::AllianceRank
            | ConstraintType::TownID
            | ConstraintType::TownPoints
            | ConstraintType::IslandID
            | ConstraintType::IslandX
            | ConstraintType::IslandY
            | ConstraintType::IslandType
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
    // TODO with this addition we need to make sure we do not send queries to the db for selections that eventually reference themselves
    InSelection,
    NotInSelection,
}

impl fmt::Display for Comparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Comparator::LessThan => write!(f, "<="),
            Comparator::Equal => write!(f, "="),
            Comparator::GreaterThan => write!(f, ">="),
            Comparator::NotEqual => write!(f, "<>"),
            Comparator::InSelection => write!(f, "IN"),
            Comparator::NotInSelection => write!(f, "NOT IN"),
        }
    }
}
