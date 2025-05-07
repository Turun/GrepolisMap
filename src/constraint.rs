use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::presenter::Presenter;
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
#[allow(clippy::struct_field_names)]
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
        presenter: &mut Presenter,
        this_selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
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
            let ddb_response = ui.add_sized(
                [
                    ui.style().spacing.interact_size.x * 4.5,
                    ui.style().spacing.interact_size.y,
                ],
                ddb,
            );
            if ddb_response.gained_focus() {
                println!("Constraint {selection_index}/{constraint_index} gained focus, refreshing drop down values list");
                self.drop_down_values = presenter
                    .drop_down_values_for_constraint(
                        &self.partial_clone(),
                        this_selection,
                        all_selections,
                    )
                    .ok();
            }
            if ddb_response.lost_focus() {
                println!("Constraint {selection_index}/{constraint_index} lost focus");
                // TODO: when losing focus we should reset self.drop_down_values to None. This prevents the
                // UI from showing a potentially incorrect list of ddv for one frame when the users clicks
                // this constraint again. The problem is to determine when we lost focus. At the moment the
                // ddb_response.has/gained/lost_focus() is only dependent on the text box, not the drop down list of
                // selectable labels. So to solve this todo we need to figure out how to reliably tell the ddb_response
                // if it has focus or not.
                // It would really be best to solve this here with the focus. But as an alternative, we could also
                // just invalidate the drop down values of all other constraints whenever one is edited. Then we
                // have the issue of not showing anything for one frame, but that is probably better than showing
                // a wrong list for one frame. On the other hand, when the values only change rarely, then showing
                // a usually-not-incorrect list would be better than showing an empty list.
                // As an alternative alternative (i.e. scratch that over eager invalidation), we could trigger the data
                // loading/refresh on hover instead of on focus. That could lead to a lot of unnecessary requests to
                // the backend if the user moves their mouse over the sidepanel (and therefore potentially lag, but that
                // would be mitigated after the first time by the backend cache). But on the other hand most users don't
                // use tab navigation, so the data will always be pre loaded correctly when they get to clicking into
                // the textbox.
            }
            if ddb_response.changed() {
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
#[derive(
    Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord,
)]
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

#[derive(
    Debug, Clone, Copy, EnumIter, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord,
)]
pub enum Comparator {
    LessThan,
    Equal,
    GreaterThan,
    NotEqual,
    InSelection,
    NotInSelection,
}

impl Comparator {
    #[allow(clippy::needless_pass_by_value)]
    pub fn compare<T: PartialEq + PartialOrd>(self, a: T, b: T) -> bool {
        match self {
            Comparator::LessThan => a <= b,
            Comparator::Equal => a == b,
            Comparator::GreaterThan => a >= b,
            Comparator::NotEqual => a != b,
            Comparator::InSelection => {
                unimplemented!("This case is never supposed to be reached. The code should handle in/notin comparators one level higher");
            }
            Comparator::NotInSelection => {
                unimplemented!("This case is never supposed to be reached. The code should handle in/notin comparators one level higher");
            }
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

#[allow(clippy::module_name_repetitions)]
pub enum ConstraintTypeType {
    StringLike,
    Number,
    IslandRessource,
    Selection,
}

impl From<&EmptyConstraint> for ConstraintTypeType {
    fn from(value: &EmptyConstraint) -> Self {
        match value.comparator {
            Comparator::LessThan
            | Comparator::Equal
            | Comparator::GreaterThan
            | Comparator::NotEqual => match value.constraint_type {
                ConstraintType::PlayerTowns
                | ConstraintType::PlayerID
                | ConstraintType::AlliancePoints
                | ConstraintType::PlayerRank
                | ConstraintType::TownPoints
                | ConstraintType::IslandID
                | ConstraintType::IslandX
                | ConstraintType::IslandY
                | ConstraintType::IslandType
                | ConstraintType::IslandTowns
                | ConstraintType::AllianceTowns
                | ConstraintType::AllianceMembers
                | ConstraintType::AllianceRank
                | ConstraintType::TownID
                | ConstraintType::PlayerPoints => Self::Number,

                ConstraintType::AllianceName
                | ConstraintType::TownName
                | ConstraintType::PlayerName => Self::StringLike,

                ConstraintType::IslandResMore | ConstraintType::IslandResLess => {
                    Self::IslandRessource
                }
            },
            Comparator::InSelection | Comparator::NotInSelection => return Self::Selection,
        }
    }
}
