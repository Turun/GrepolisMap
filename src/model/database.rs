use std::collections::HashSet;
use std::ops::Deref;
use std::rc::Rc;

use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::model::ConstraintType;
use crate::town::Town;

pub struct Offset {
    pub typ: u8,
    pub x: u16,
    pub y: u16,
    pub slot_number: u8,
}

pub struct Island {
    pub id: u32,
    pub x: u16,
    pub y: u16,
    pub typ: u8,
    pub towns: u8,
    pub ressource_plus: String,
    pub ressource_minus: String,
}

pub struct Alliance {
    pub id: u32,
    pub name: String,
    pub points: u32,
    pub towns: u32,
    pub members: u16,
    pub rank: u16,
}

pub struct Player {
    pub id: u32,
    pub name: String,
    pub alliance: Option<(u32, Rc<Alliance>)>, // link player.alliance_id == alliance.id
    pub points: u32,
    pub rank: u16,
    pub towns: u16,
}

// TODO: Merge BackendTown and Town as it is used for the frontend into one struct

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

impl From<&BackendTown> for Town {
    fn from(value: &BackendTown) -> Self {
        Self {
            id: value.id as i32,
            player_id: value.player.map(|(id, _)| id as i32),
            player_name: value.player.map(|(_, p)| p.name),
            alliance_name: value
                .player
                .map(|(_, p)| p.alliance)
                .flatten()
                .map(|(_, a)| a.name),
            name: value.name,
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
        constraints: &[EmptyConstraint],
        join_mode: &str,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Vec<String>> {
        if constraints.is_empty() {
            return self.get_names_for_constraint_type(constraint_type);
        }

        let towns =
            self.get_backendtowns_for_constraints(constraints, join_mode, all_selections)?;
        return get_names_for_constraint_type_in_town_list(&towns, constraint_type);
    }

    pub fn get_towns_for_constraints(
        &self,
        constraints: &[EmptyConstraint],
        join_mode: &str,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Vec<Town>> {
        if constraints.is_empty() {
            return Ok(Vec::new());
        }

        return Ok(self
            .get_backendtowns_for_constraints(constraints, join_mode, all_selections)?
            .iter()
            .map(|bt| &**bt)
            .map(|bt| bt.into())
            .collect());
    }

    pub fn get_backendtowns_for_constraints(
        &self,
        constraints: &[EmptyConstraint],
        join_mode: &str,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Vec<Rc<BackendTown>>> {
        if constraints.is_empty() {
            return Ok(Vec::new());
        }

        Ok(self
            .towns
            .iter()
            .filter(|t| match join_mode {
                "AND" => constraints
                    .iter()
                    .all(|c| c.matches(&&***t, all_selections)),
                "OR" => constraints
                    .iter()
                    .any(|c| c.matches(&&***t, all_selections)),
                _ => {
                    unreachable!()
                }
            })
            .cloned()
            .collect())
    }
}

pub fn get_names_for_constraint_type_in_town_list(
    towns: &[Rc<BackendTown>],
    constraint_type: ConstraintType,
) -> anyhow::Result<Vec<String>> {
    let mut re = HashSet::new();
    for t in towns {
        let opt_value: Option<String> = match constraint_type {
            ConstraintType::PlayerID => t.player.map(|(id, _)| id).map(|x| format!("{x}")),
            ConstraintType::PlayerName => t.player.map(|(_id, player)| player.name),
            ConstraintType::PlayerPoints => t
                .player
                .map(|(_id, player)| player.points)
                .map(|x| format!("{x}")),
            ConstraintType::PlayerRank => t
                .player
                .map(|(_id, player)| player.rank)
                .map(|x| format!("{x}")),
            ConstraintType::PlayerTowns => t
                .player
                .map(|(_id, player)| player.towns)
                .map(|x| format!("{x}")),
            ConstraintType::AllianceName => t
                .player
                .map(|(_id, player)| player.alliance)
                .flatten()
                .map(|(_id, alliance)| alliance.name),
            ConstraintType::AlliancePoints => t
                .player
                .map(|(_id, player)| player.alliance)
                .flatten()
                .map(|(_id, alliance)| alliance.points)
                .map(|x| format!("{x}")),
            ConstraintType::AllianceTowns => t
                .player
                .map(|(_id, player)| player.alliance)
                .flatten()
                .map(|(_id, alliance)| alliance.towns)
                .map(|x| format!("{x}")),
            ConstraintType::AllianceMembers => t
                .player
                .map(|(_id, player)| player.alliance)
                .flatten()
                .map(|(_id, alliance)| alliance.members)
                .map(|x| format!("{x}")),
            ConstraintType::AllianceRank => t
                .player
                .map(|(_id, player)| player.alliance)
                .flatten()
                .map(|(_id, alliance)| alliance.rank)
                .map(|x| format!("{x}")),
            ConstraintType::TownID => Some(t.id).map(|x| format!("{x}")),
            ConstraintType::TownName => Some(t.name),
            ConstraintType::TownPoints => Some(t.id).map(|x| format!("{x}")),
            ConstraintType::IslandID => {
                let (_x, _y, island) = t.island;
                Some(island.id).map(|x| format!("{x}"))
            }
            ConstraintType::IslandX => {
                let (x, _y, _island) = t.island;
                Some(x).map(|x| format!("{x}"))
            }
            ConstraintType::IslandY => {
                let (_x, y, _island) = t.island;
                Some(y).map(|x| format!("{x}"))
            }
            ConstraintType::IslandType => {
                let (_x, _y, island) = t.island;
                Some(island.typ).map(|x| format!("{x}"))
            }
            ConstraintType::IslandTowns => {
                let (_x, _y, island) = t.island;
                Some(island.towns).map(|x| format!("{x}"))
            }
            ConstraintType::IslandResMore => {
                let (_x, _y, island) = t.island;
                Some(island.ressource_plus).map(|x| format!("{x}"))
            }
            ConstraintType::IslandResLess => {
                let (_x, _y, island) = t.island;
                Some(island.ressource_minus).map(|x| format!("{x}"))
            }
        };

        if let Some(value) = opt_value {
            let _duplicate = re.insert(value);
        }
    }

    return Ok(re.into_iter().collect());
}
