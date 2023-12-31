use crate::emptyconstraint::EmptyConstraint;
use crate::selection::AndOr;
use crate::view::dropdownbox::DropDownBox;
use crate::view::Change;
use egui::{Button, Label};
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
            self.constraint_type.to_string(),
            self.comparator.to_string(),
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
        and_or: AndOr,
    ) -> (Option<Change>, bool, bool) {
        let mut re_edited = false;
        let mut re_change = None;
        let mut re_and_or_toggled = false;

        ui.horizontal(|ui| {
            // Filter for which attribute?
            let _inner_response = egui::ComboBox::from_id_source(format!(
                "ComboxBox {selection_index}/{constraint_index} Type"
            ))
            .width(ui.style().spacing.interact_size.x * 3.5)
            .selected_text(self.constraint_type.to_string())
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
            .selected_text(self.comparator.to_string())
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
            let first_item = constraint_index == 0;
            let and_or_plus_size = [
                ui.style().spacing.interact_size.x * 1.0,
                ui.style().spacing.interact_size.y,
            ];
            if last_item {
                let button = Button::new("+");
                if ui.add_sized(and_or_plus_size, button).clicked() {
                    re_change = Some(Change::Add);
                }
            } else if first_item {
                let button = Button::new(and_or.to_string());
                if ui.add_sized(and_or_plus_size, button).clicked() {
                    re_and_or_toggled = true;
                    re_edited = true;
                }
            } else {
                let label = Label::new(and_or.to_string());
                ui.add_sized(and_or_plus_size, label);
            }
            if ui.button(" - ").clicked() {
                re_change = Some(Change::Remove(constraint_index));
            }
            if ui.button("↑").clicked() {
                re_change = Some(Change::MoveUp(constraint_index));
            }
            if ui.button("↓").clicked() {
                re_change = Some(Change::MoveDown(constraint_index));
            }
        });

        (re_change, re_edited, re_and_or_toggled)
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

impl ToString for ConstraintType {
    fn to_string(&self) -> String {
        match self {
            ConstraintType::PlayerID => t!("selection.constraint.player_id"),
            ConstraintType::PlayerName => t!("selection.constraint.player_name"),
            ConstraintType::PlayerPoints => t!("selection.constraint.player_points"),
            ConstraintType::PlayerRank => t!("selection.constraint.player_rank"),
            ConstraintType::PlayerTowns => t!("selection.constraint.player_towns"),
            ConstraintType::AllianceName => t!("selection.constraint.alliance_name"),
            ConstraintType::AlliancePoints => t!("selection.constraint.alliance_points"),
            ConstraintType::AllianceTowns => t!("selection.constraint.alliance_towns"),
            ConstraintType::AllianceMembers => t!("selection.constraint.alliance_members"),
            ConstraintType::AllianceRank => t!("selection.constraint.alliance_rank"),
            ConstraintType::TownID => t!("selection.constraint.town_id"),
            ConstraintType::TownName => t!("selection.constraint.town_name"),
            ConstraintType::TownPoints => t!("selection.constraint.town_points"),
            ConstraintType::IslandID => t!("selection.constraint.island_id"),
            ConstraintType::IslandX => t!("selection.constraint.island_x"),
            ConstraintType::IslandY => t!("selection.constraint.island_y"),
            ConstraintType::IslandType => t!("selection.constraint.island_type"),
            ConstraintType::IslandTowns => t!("selection.constraint.island_towns"),
            ConstraintType::IslandResMore => t!("selection.constraint.island_resmore"),
            ConstraintType::IslandResLess => t!("selection.constraint.island_resless"),
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
    InSelection,
    NotInSelection,
}

impl Comparator {
    pub fn as_sql(self) -> String {
        match self {
            Comparator::LessThan => "<=".to_string(),
            Comparator::Equal => "=".to_string(),
            Comparator::GreaterThan => ">=".to_string(),
            Comparator::NotEqual => "<>".to_string(),
            Comparator::InSelection => "IN".to_string(),
            Comparator::NotInSelection => "NOT IN".to_string(),
        }
    }
}

impl ToString for Comparator {
    fn to_string(&self) -> String {
        match self {
            Comparator::LessThan => "<=".to_string(),
            Comparator::Equal => "=".to_string(),
            Comparator::GreaterThan => ">=".to_string(),
            Comparator::NotEqual => "!=".to_string(),
            Comparator::InSelection => t!("selection.comparator.in"),
            Comparator::NotInSelection => t!("selection.comparator.not_in"),
        }
    }
}
