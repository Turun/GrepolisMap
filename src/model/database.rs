use std::collections::HashSet;
use std::hash::Hash;
use std::ops::Deref;
use std::rc::Rc;

use crate::emptyconstraint::EmptyConstraint;
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
    pub points: u16,
    pub player: Option<(u32, Rc<Player>)>, // link town.player_id == player.id
    pub island: (u16, u16, Rc<Island>),    // link town.x = island.x && town.y == island.y
    pub offset: (u8, Rc<Offset>), // link town.slot_number = offset.slot_number && offset.type == island.type
    pub actual_x: f32,
    pub actual_y: f32, // computed from the linked island and offset
}
impl PartialEq for BackendTown {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for BackendTown {}
impl std::hash::Hash for BackendTown {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl From<&BackendTown> for Town {
    fn from(value: &BackendTown) -> Self {
        Self {
            id: value.id as i32,
            player_id: value.player.as_ref().map(|(id, _)| *id as i32),
            player_name: value.player.as_ref().map(|(_, p)| p.name.clone()),
            alliance_name: value
                .player
                .as_ref()
                .map(|(_, p)| p.alliance.as_ref())
                .flatten()
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
    pub fn get_all_towns(&self) -> anyhow::Result<Vec<Town>> {
        Ok(self.towns.iter().map(|t| t.deref().into()).collect())
    }

    pub fn get_ghost_towns(&self) -> anyhow::Result<Vec<Town>> {
        Ok(self
            .towns
            .iter()
            .map(|t| t.deref())
            .filter(|t| t.player.is_none())
            .map(|t| t.into())
            .collect())
    }

    pub fn get_names_for_constraint_type(
        &self,
        constraint_type: ConstraintType,
    ) -> anyhow::Result<Vec<String>> {
        return get_names_for_constraint_type_in_town_list(&self.towns, constraint_type);
    }

    pub fn get_names_for_constraint_type_in_constraints(
        &self,
        constraint_type: ConstraintType,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Vec<String>> {
        if selection.constraints.is_empty() {
            return self.get_names_for_constraint_type(constraint_type);
        }

        let towns = self.get_backendtowns_for_constraints(&selection, all_selections)?;
        return get_names_for_constraint_type_in_town_list(&towns, constraint_type);
    }

    pub fn get_towns_for_constraints(
        &self,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Vec<Town>> {
        if selection.constraints.is_empty() {
            return Ok(Vec::new());
        }

        return Ok(self
            .get_backendtowns_for_constraints(selection, all_selections)?
            .iter()
            .map(|bt| &**bt)
            .map(|bt| bt.into())
            .collect());
    }

    pub fn get_backendtowns_for_constraints(
        &self,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Vec<Rc<BackendTown>>> {
        if selection.constraints.is_empty() {
            return Ok(Vec::new());
        }

        let re = super::database::matching_towns_for_selection(
            HashSet::from_iter(self.towns.clone().into_iter()),
            &selection,
            all_selections,
        )
        .into_iter()
        .collect();

        return Ok(re);
    }
}

pub fn matching_towns_for_selection(
    towns: HashSet<Rc<BackendTown>>,
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

pub fn get_names_for_constraint_type_in_town_list(
    towns: &[Rc<BackendTown>],
    constraint_type: ConstraintType,
) -> anyhow::Result<Vec<String>> {
    // TODO: this should really be sorted by value. for now we sort by resulting string, but we really should pull the collection and sorting into each match branch
    let mut re = HashSet::new();
    for t in towns {
        let opt_value: Option<String> = match constraint_type {
            ConstraintType::PlayerID => t.player.as_ref().map(|(id, _)| id).map(|x| format!("{x}")),
            ConstraintType::PlayerName => {
                t.player.as_ref().map(|(_id, player)| player.name.clone())
            }
            ConstraintType::PlayerPoints => t
                .player
                .as_ref()
                .map(|(_id, player)| player.points)
                .map(|x| format!("{x}")),
            ConstraintType::PlayerRank => t
                .player
                .as_ref()
                .map(|(_id, player)| player.rank)
                .map(|x| format!("{x}")),
            ConstraintType::PlayerTowns => t
                .player
                .as_ref()
                .map(|(_id, player)| player.towns)
                .map(|x| format!("{x}")),
            ConstraintType::AllianceName => t
                .player
                .as_ref()
                .map(|(_id, player)| player.alliance.clone())
                .flatten()
                .map(|(_id, alliance)| alliance.name.clone()),
            ConstraintType::AlliancePoints => t
                .player
                .as_ref()
                .map(|(_id, player)| player.alliance.clone())
                .flatten()
                .map(|(_id, alliance)| alliance.points)
                .map(|x| format!("{x}")),
            ConstraintType::AllianceTowns => t
                .player
                .as_ref()
                .map(|(_id, player)| player.alliance.clone())
                .flatten()
                .map(|(_id, alliance)| alliance.towns)
                .map(|x| format!("{x}")),
            ConstraintType::AllianceMembers => t
                .player
                .as_ref()
                .map(|(_id, player)| player.alliance.clone())
                .flatten()
                .map(|(_id, alliance)| alliance.members)
                .map(|x| format!("{x}")),
            ConstraintType::AllianceRank => t
                .player
                .as_ref()
                .map(|(_id, player)| player.alliance.clone())
                .flatten()
                .map(|(_id, alliance)| alliance.rank)
                .map(|x| format!("{x}")),
            ConstraintType::TownID => Some(t.id).map(|x| format!("{x}")),
            ConstraintType::TownName => Some(t.name.clone()),
            ConstraintType::TownPoints => Some(t.id).map(|x| format!("{x}")),
            ConstraintType::IslandID => {
                let (_x, _y, island) = &t.island;
                Some(island.id).map(|x| format!("{x}"))
            }
            ConstraintType::IslandX => {
                let (x, _y, _island) = &t.island;
                Some(x).map(|x| format!("{x}"))
            }
            ConstraintType::IslandY => {
                let (_x, y, _island) = &t.island;
                Some(y).map(|x| format!("{x}"))
            }
            ConstraintType::IslandType => {
                let (_x, _y, island) = &t.island;
                Some(island.typ).map(|x| format!("{x}"))
            }
            ConstraintType::IslandTowns => {
                let (_x, _y, island) = &t.island;
                Some(island.towns).map(|x| format!("{x}"))
            }
            ConstraintType::IslandResMore => {
                let (_x, _y, island) = &t.island;
                Some(island.ressource_plus.clone())
            }
            ConstraintType::IslandResLess => {
                let (_x, _y, island) = &t.island;
                Some(island.ressource_minus.clone())
            }
        };

        if let Some(value) = opt_value {
            let _duplicate = re.insert(value);
        }
    }
    let mut re = re.into_iter().collect::<Vec<_>>();
    re.sort();

    return Ok(re);
}
