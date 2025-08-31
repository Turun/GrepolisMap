use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;

use crate::emptyselection::EmptyTownSelection;
use crate::model::ConstraintType;
use crate::selection::AndOr;
use crate::town::Town;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Offset {
    pub typ: u8,
    pub x: u16,
    pub y: u16,
    pub slot_number: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Island {
    pub id: u32,
    pub x: u16,
    pub y: u16,
    pub typ: u8,
    pub towns: u8,
    pub ressource_plus: String,
    pub ressource_minus: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Alliance {
    pub id: u32,
    pub name: String,
    pub points: u32,
    pub towns: u32,
    pub members: u16,
    pub rank: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Player {
    pub id: u32,
    pub name: String,
    pub alliance: Option<(u32, Rc<Alliance>)>, // link player.alliance_id == alliance.id
    pub points: u32,
    pub rank: u16,
    pub towns: u16,
}

// TODO: Merge BackendTown and Town as it is used for the frontend into one struct

#[derive(Debug, Clone)]
pub struct BackendTown {
    pub id: u32,
    pub name: String,
    pub points: u16, // had a bug where a city actually had negative points in the game
    pub player: Option<(u32, Rc<Player>)>, // link town.player_id == player.id
    pub island: (u16, u16, Rc<Island>), // link town.x = island.x && town.y == island.y
    pub offset: (u8, Rc<Offset>), // link town.slot_number = offset.slot_number && offset.type == island.type
    pub actual_x: f32,
    pub actual_y: f32, // computed from the linked island and offset
}
impl Eq for BackendTown {}
impl PartialEq for BackendTown {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl std::hash::Hash for BackendTown {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl From<&BackendTown> for Town {
    fn from(value: &BackendTown) -> Self {
        Self {
            id: value.id,
            player_id: value.player.as_ref().map(|(id, _)| *id),
            player_name: value.player.as_ref().map(|(_, p)| p.name.clone()),
            alliance_name: value
                .player
                .as_ref()
                .and_then(|(_, p)| p.alliance.as_ref())
                .map(|(_, a)| a.name.clone()),
            name: value.name.clone(),
            x: value.actual_x,
            y: value.actual_y,
            slot_number: value.offset.1.slot_number,
            points: value.points,
        }
    }
}

pub struct DataTable {
    pub towns: Vec<Rc<BackendTown>>,
}

impl DataTable {
    pub fn get_all_towns(&self) -> Vec<Town> {
        self.towns.iter().map(|t| t.deref().into()).collect()
    }

    pub fn get_ghost_towns(&self) -> Vec<Town> {
        self.towns
            .iter()
            .map(std::ops::Deref::deref)
            .filter(|t| t.player.is_none())
            .map(std::convert::Into::into)
            .collect()
    }

    pub fn get_names_for_constraint_type(&self, constraint_type: ConstraintType) -> Vec<String> {
        return get_names_for_constraint_type_in_town_list(&self.towns, constraint_type);
    }

    pub fn get_names_for_constraint_type_in_constraints(
        &self,
        constraint_type: ConstraintType,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> Vec<String> {
        if selection.constraints.is_empty() {
            return self.get_names_for_constraint_type(constraint_type);
        }

        let towns = self.get_backendtowns_for_constraints(selection, all_selections);
        return get_names_for_constraint_type_in_town_list(&towns, constraint_type);
    }

    pub fn get_towns_for_constraints(
        &self,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> Vec<Town> {
        if selection.constraints.is_empty() {
            return Vec::new();
        }

        return self
            .get_backendtowns_for_constraints(selection, all_selections)
            .iter()
            .map(|bt| &**bt)
            .map(std::convert::Into::into)
            .collect();
    }

    pub fn get_backendtowns_for_constraints(
        &self,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> Vec<Rc<BackendTown>> {
        if selection.constraints.is_empty() {
            return Vec::new();
        }

        return matching_towns_for_selection(
            &HashSet::from_iter(self.towns.clone()),
            selection,
            all_selections,
        )
        .into_iter()
        .collect();
    }
}

pub fn matching_towns_for_selection(
    towns: &HashSet<Rc<BackendTown>>,
    selection: &EmptyTownSelection,
    all_selections: &[EmptyTownSelection],
) -> HashSet<Rc<BackendTown>> {
    // short circuit useless selections.
    // useless selection in this case means that for all constraints where a value is provided by the user the input must be valid
    if !selection
        .constraints
        .iter()
        .filter(|c| !c.value.is_empty())
        .all(|c| c.has_valid_input(all_selections))
    {
        return HashSet::new();
    }

    let mut local_towns = match selection.constraint_join_mode {
        AndOr::And => towns.clone(), // for and we need to start with the full set and widdle it down
        AndOr::Or => HashSet::new(), // for or we need to start with nothing and build it up gradually
    };

    // for all valid constraints
    for constraint in selection
        .constraints
        .iter()
        .filter(|c| c.has_valid_input(all_selections))
    {
        // add/remove towns to/from the inital list based on if they match the constraint or not
        match selection.constraint_join_mode {
            AndOr::And => {
                // shortcut dataprocessing. AND join means that we can never reintroduce towns that were already excluded by another constraint
                constraint.matching_towns(
                    &mut local_towns,
                    all_selections,
                    selection.constraint_join_mode,
                );
            }
            AndOr::Or => {
                // for OR joining we need to do the full list with every constraint
                let mut these_towns = towns.clone();

                constraint.matching_towns(
                    &mut these_towns,
                    all_selections,
                    selection.constraint_join_mode,
                );

                local_towns.extend(these_towns.into_iter());
            }
        }
    }

    return local_towns;
}

#[allow(clippy::too_many_lines)]
pub fn get_names_for_constraint_type_in_town_list(
    towns: &[Rc<BackendTown>],
    constraint_type: ConstraintType,
) -> Vec<String> {
    // This is a big chunk of the actual work the program is doing. If we want to speed it up, we could
    // - cache the format! calls
    //
    // It does not help to
    // - turn the collect::<Vec> -> sort -> dedup chain into a BTreeSet. I tested it

    return match constraint_type {
        ConstraintType::PlayerID => {
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .map(|(id, _player)| id)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::PlayerName => {
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .map(|(_id, player)| player.name.clone())
                .collect::<Vec<_>>();
            values.sort_unstable_by_key(|k| k.to_lowercase());
            values.dedup();
            values
        }
        ConstraintType::PlayerPoints => {
            // todo
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .map(|(_id, player)| player.points)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::PlayerRank => {
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .map(|(_id, player)| player.rank)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::PlayerTowns => {
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .map(|(_id, player)| player.towns)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::AllianceName => {
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .filter_map(|(_id, player)| player.alliance.as_ref())
                .map(|(_id, ally)| ally.name.clone())
                .collect::<Vec<_>>();
            values.sort_unstable_by_key(|k| k.to_lowercase());
            values.dedup();
            values
        }
        ConstraintType::AlliancePoints => {
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .filter_map(|(_id, player)| player.alliance.as_ref())
                .map(|(_id, ally)| ally.points)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::AllianceTowns => {
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .filter_map(|(_id, player)| player.alliance.as_ref())
                .map(|(_id, ally)| ally.towns)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::AllianceMembers => {
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .filter_map(|(_id, player)| player.alliance.as_ref())
                .map(|(_id, ally)| ally.members)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::AllianceRank => {
            let mut values = towns
                .iter()
                .filter_map(|t| t.player.as_ref())
                .filter_map(|(_id, player)| player.alliance.as_ref())
                .map(|(_id, ally)| ally.rank)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::TownID => {
            let mut values = towns.iter().map(|t| t.id).collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::TownName => {
            let mut values = towns.iter().map(|t| t.name.clone()).collect::<Vec<_>>();
            values.sort_unstable_by_key(|s| s.to_lowercase());
            values.dedup();
            values
        }
        ConstraintType::TownPoints => {
            let mut values = towns.iter().map(|t| t.points).collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::IslandID => {
            let mut values = towns
                .iter()
                .map(|t| t.island.clone())
                .map(|(_x, _y, island)| island.id)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::IslandX => {
            let mut values = towns
                .iter()
                .map(|t| t.island.clone())
                .map(|(x, _y, _island)| x)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::IslandY => {
            let mut values = towns
                .iter()
                .map(|t| t.island.clone())
                .map(|(_x, y, _island)| y)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::IslandType => {
            let mut values = towns
                .iter()
                .map(|t| t.island.clone())
                .map(|(_x, _y, island)| island.typ)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::IslandTowns => {
            let mut values = towns
                .iter()
                .map(|t| t.island.clone())
                .map(|(_x, _y, island)| island.towns)
                .collect::<Vec<_>>();
            values.sort_unstable();
            values.dedup();
            values.iter().map(|x| format!("{x}")).collect::<Vec<_>>()
        }
        ConstraintType::IslandResMore => {
            // todo
            let mut values = towns
                .iter()
                .map(|t| t.island.clone())
                .map(|(_x, _y, island)| island.ressource_plus.clone())
                .collect::<Vec<_>>();
            values.sort_unstable_by_key(|s| s.to_lowercase());
            values.dedup();
            values
        }
        ConstraintType::IslandResLess => {
            // todo
            let mut values = towns
                .iter()
                .map(|t| t.island.clone())
                .map(|(_x, _y, island)| island.ressource_minus.clone())
                .collect::<Vec<_>>();
            values.sort_unstable_by_key(|s| s.to_lowercase());
            values.dedup();
            values
        }
    };
}
