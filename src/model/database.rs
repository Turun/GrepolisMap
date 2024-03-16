use std::collections::HashMap;

use anyhow::{Context, Result};
use runtime_format::FormatArgs;
use rusqlite::Statement;

use crate::constraint::Comparator;
use crate::emptyconstraint::EmptyConstraint;
use crate::emptyselection::EmptyTownSelection;
use crate::model::ConstraintType;
use crate::town::Town;

pub struct RowOffset {
    pub offset_type: u8,
    pub offset_x: u16,
    pub offset_y: u16,
    pub offset_slot_number: u8,
}

pub struct RowIsland {
    pub island_id: u32,
    pub island_x: u16,
    pub island_y: u16,
    pub island_type: u8,
    pub island_towns: u8,
    pub island_ressource_plus: String,
    pub island_ressource_minus: String,
}

pub struct RowTown {
    pub town_id: u32,
    pub town_name: String,
    pub town_points: u16,
    pub town_player: Option<(u32, RowPlayer)>, // link town.player_id == player.id
    pub town_island: (u32, u32, RowIsland),    // link town.x = island.y && town.y == island.y
    pub town_offset: (u32, RowOffset), // link town.slot_number = offset.slot_number && offset.type == island.type
}

pub struct RowAlliance {
    pub alliance_id: u32,
    pub alliance_name: String,
    pub alliance_points: u32,
    pub alliance_towns: u32,
    pub alliance_members: u16,
    pub alliance_rank: u16,
}

pub struct RowPlayer {
    pub player_id: u32,
    pub player_name: String,
    pub player_alliance: Option<(u32, RowAlliance)>, // link player.alliance_id == alliance.id
    pub player_points: u32,
    pub player_rank: u16,
    pub player_towns: u16,
}

pub struct DataTable {
    towns: Vec<RowTown>,
}

pub struct Database {
    pub connection: rusqlite::Connection,
}

pub trait ToSqlFragment {
    fn to_sql_fragment(&self, parameter_index: usize) -> String;
}

struct GhostTown;
impl ToSqlFragment for GhostTown {
    fn to_sql_fragment(&self, _parameter_index: usize) -> String {
        "towns.player_id IS NULL".into()
    }
}

struct AllTowns;
impl ToSqlFragment for AllTowns {
    fn to_sql_fragment(&self, _parameter_index: usize) -> String {
        "true".into()
    }
}

impl ToSqlFragment for EmptyConstraint {
    fn to_sql_fragment(&self, parameter_index: usize) -> String {
        match self.comparator {
            Comparator::LessThan
            | Comparator::Equal
            | Comparator::GreaterThan
            | Comparator::NotEqual => {
                format!(
                    "{}.{} {} ?{}",
                    self.constraint_type.table(),
                    self.constraint_type.property(),
                    self.comparator.as_sql(),
                    parameter_index + 1
                )
            }
            Comparator::InSelection | Comparator::NotInSelection => {
                format!(
                    "{}.{} {} ({{{}}})",
                    self.constraint_type.table(),
                    self.constraint_type.property(),
                    self.comparator.as_sql(),
                    parameter_index + 1
                )
            }
        }
    }
}

static TOWN_SELECTION: &str =
    "towns.*, offsets.offset_x, offsets.offset_y, players.name, alliances.name";

impl Database {
    fn construct_sql<SQL>(
        selection_clause: &str,
        filter_clauses: &[SQL],
        join_mode: &str,
        order_clause: Option<&str>,
    ) -> String
    where
        SQL: ToSqlFragment,
    {
        let sql_start = format!(
            "SELECT {selection_clause} from \n\
                towns \n\
                LEFT JOIN islands ON (towns.island_x = islands.x AND towns.island_y = islands.y) \n\
                LEFT JOIN offsets ON (towns.slot_number = offsets.slot_number) \n\
                LEFT JOIN players ON (towns.player_id = players.player_id) \n\
                LEFT JOIN alliances ON (players.alliance_id = alliances.alliance_id) \n\
                WHERE islands.type = offsets.type AND \n",
        );

        let sql_fragments = filter_clauses
            .iter()
            .enumerate()
            .map(|(i, x)| x.to_sql_fragment(i))
            .collect::<Vec<String>>()
            .join(&format!(" \n{join_mode} "));

        let sql_order = if let Some(text) = order_clause {
            String::from(" \n") + text
        } else {
            String::new()
        };
        // join the different parts together. The parentheses are important, because
        // they ensure the order of precedence does not mingle the island type join
        // with the user defined constraints.
        sql_start + "(" + &sql_fragments + ")" + &sql_order
    }

    fn sql_to_prepared_statement(&self, sql: &str) -> anyhow::Result<Statement> {
        self.connection
            .prepare(sql)
            .context("Failed to get towns from database (build statement)")
    }

    fn sql_to_bound_statement<'a>(
        &'a self,
        sql_text: &str,
        constraints: &[EmptyConstraint],
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<Statement<'a>> {
        let mut format_mapping = HashMap::new();
        for (index, constraint) in constraints.iter().enumerate() {
            if constraint.referenced_selection().is_some() {
                format_mapping.insert(
                    (index + 1).to_string(),
                    constraint.get_sql_value(self, all_selections),
                );
            }
        }
        let sql_text = FormatArgs::new(sql_text, &format_mapping).to_string();
        let mut statement = self.sql_to_prepared_statement(&sql_text)?;

        for (index, constraint) in constraints.iter().enumerate() {
            if constraint.referenced_selection().is_none() {
                statement.raw_bind_parameter(
                    index + 1,
                    &constraint.get_sql_value(self, all_selections),
                )?;
            }
        }
        Ok(statement)
    }

    pub fn selection_to_sql(
        &self,
        selection_clause: &str,
        selection: &EmptyTownSelection,
        all_selections: &[EmptyTownSelection],
    ) -> anyhow::Result<String> {
        let sql = Self::construct_sql(
            selection_clause,
            &selection.constraints,
            &selection.constraint_join_mode.as_sql(),
            None,
        );
        let statement =
            self.sql_to_bound_statement(&sql, &selection.constraints, all_selections)?;
        statement.expanded_sql().ok_or(anyhow::Error::msg(
            "Failed to convert the selection {selection} into an SQL string",
        ))
    }

    pub fn get_all_towns(&self) -> anyhow::Result<Vec<Town>> {
        let sql = Self::construct_sql(TOWN_SELECTION, &[AllTowns], "and", None);
        let mut statement = self.sql_to_prepared_statement(&sql)?;
        let rows = statement
            .query([])
            .context("Failed to get towns from the database (perform query)")?
            .mapped(Town::from)
            .inspect(|res_t| {
                if let Err(err) = res_t {
                    eprintln!("{err:?}")
                }
            })
            .filter_map(|res_t| res_t.ok())
            .collect();

        Ok(rows)
    }

    pub fn get_ghost_towns(&self) -> anyhow::Result<Vec<Town>> {
        let sql = Self::construct_sql(TOWN_SELECTION, &[GhostTown], "and", None);
        let mut statement = self.sql_to_prepared_statement(&sql)?;
        let rows = statement
            .query([])
            .context("Failed to get ghost towns from the database (perform query)")?
            .mapped(Town::from)
            .inspect(|res_t| {
                if let Err(err) = res_t {
                    eprintln!("{err:?}")
                }
            })
            .filter_map(|res_t| res_t.ok())
            .collect();

        Ok(rows)
    }

    pub fn get_names_for_constraint_type(
        &self,
        constraint_type: ConstraintType,
    ) -> anyhow::Result<Vec<String>> {
        let ct_property = constraint_type.property();
        let ct_table = constraint_type.table();

        let statement_text = if constraint_type.is_string() {
            format!(
                "SELECT DISTINCT {ct_table}.{ct_property} from {ct_table} ORDER BY LOWER({ct_table}.{ct_property})",
            )
        } else {
            format!(
                "SELECT DISTINCT {ct_table}.{ct_property} from {ct_table} ORDER BY {ct_table}.{ct_property}"
            )
        };

        let mut statement = self.sql_to_prepared_statement(&statement_text)?;
        let rows = statement
            .query([])
            .context("Failed to get names from the database (perform query)")?
            .mapped(|row| {
                if constraint_type.is_string() {
                    row.get::<usize, String>(0)
                } else {
                    row.get::<usize, usize>(0).map(|value| format!("{value}"))
                }
            })
            .collect::<std::result::Result<Vec<String>, rusqlite::Error>>()
            .context("Failed to collect names from rows")?;

        Ok(rows)
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

        let ct_property = constraint_type.property();
        let ct_table = constraint_type.table();
        let order_clause = if constraint_type.is_string() {
            format!("ORDER BY LOWER({ct_table}.{ct_property})")
        } else {
            format!("ORDER BY {ct_table}.{ct_property}")
        };
        let sql = Self::construct_sql(
            &format!("DISTINCT {ct_table}.{ct_property}"),
            constraints,
            join_mode,
            Some(&order_clause),
        );

        let mut statement = self.sql_to_bound_statement(&sql, constraints, all_selections)?;
        let rows = statement
            .raw_query()
            .mapped(|row| {
                if constraint_type.is_string() {
                    row.get::<usize, String>(0)
                } else {
                    let value_option = row.get::<usize, usize>(0);
                    match value_option {
                        Ok(value) => Ok(format!("{value}")),
                        Err(err) => {
                            eprintln!("{err:?}");
                            Err(err)
                        }
                    }
                }
            })
            .filter_map(Result::ok)
            .collect();

        Ok(rows)
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

        let sql = Self::construct_sql(TOWN_SELECTION, constraints, join_mode, None);
        let mut statement = self.sql_to_bound_statement(&sql, constraints, all_selections)?;
        let rows = statement
            .raw_query()
            .mapped(Town::from)
            .inspect(|res_t| {
                if let Err(err) = res_t {
                    eprintln!("{err:?}")
                }
            })
            .filter_map(|res_t| res_t.ok())
            .collect();

        Ok(rows)
    }
}
