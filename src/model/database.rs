use anyhow::{Context, Result};
use rusqlite::{self, types::ToSqlOutput, ToSql};

use crate::model::Constraint;
use crate::model::ConstraintType;
use crate::town::Town;

pub struct Database {
    pub connection: rusqlite::Connection,
}

enum EitherOr {
    A(String),
    B(usize),
}

impl ToSql for EitherOr {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        match self {
            EitherOr::A(value) => value.to_sql(),
            EitherOr::B(value) => value.to_sql(),
        }
    }
}

impl Database {
    pub fn get_all_towns(&self) -> anyhow::Result<Vec<Town>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT towns.*, offsets.offset_x, offsets.offset_y, players.name, alliances.name from 
                towns 
                LEFT JOIN islands ON (towns.island_x = islands.x AND towns.island_y = islands.y)
                LEFT JOIN offsets ON (towns.slot_number = offsets.slot_number)
                LEFT JOIN players ON (towns.player_id = players.player_id)
                LEFT JOIN alliances ON (players.alliance_id = alliances.alliance_id)
                WHERE islands.type = offsets.type",
            )
            .context("Failed to get towns from database (build statement)")?;
        let rows = statement
            .query([])
            .context("Failed to get towns from the database (perform query)")?
            .mapped(Town::from)
            .collect::<std::result::Result<Vec<Town>, rusqlite::Error>>()
            .context("Failed to create a town from row")?;

        Ok(rows)
    }

    pub fn get_ghost_towns(&self) -> anyhow::Result<Vec<Town>> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT towns.*, offsets.offset_x, offsets.offset_y, players.name, alliances.name from 
                towns 
                LEFT JOIN islands ON (towns.island_x = islands.x AND towns.island_y = islands.y)
                LEFT JOIN offsets ON (towns.slot_number = offsets.slot_number)
                LEFT JOIN players ON (towns.player_id = players.player_id)
                LEFT JOIN alliances ON (players.alliance_id = alliances.alliance_id)
                WHERE islands.type = offsets.type AND towns.player_id IS NULL",
            )
            .context("Failed to get ghost towns from database (build statement)")?;
        let rows = statement
            .query([])
            .context("Failed to get ghost towns from the database (perform query)")?
            .mapped(Town::from)
            .collect::<std::result::Result<Vec<Town>, rusqlite::Error>>()
            .context("Failed to create a town from row")?;

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

        let mut statement = self
            .connection
            .prepare(&statement_text)
            .context("Failed to get names from database (build statement)")?;
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
        constraints: &[Constraint],
    ) -> anyhow::Result<Vec<String>> {
        if constraints.is_empty() {
            return self.get_names_for_constraint_type(constraint_type);
        }

        let ct_property = constraint_type.property();
        let ct_table = constraint_type.table();
        let mut statement_text = format!(
            "SELECT DISTINCT {ct_table}.{ct_property} from 
                towns 
                LEFT JOIN islands ON (towns.island_x = islands.x AND towns.island_y = islands.y)
                LEFT JOIN offsets ON (towns.slot_number = offsets.slot_number)
                LEFT JOIN players ON (towns.player_id = players.player_id)
                LEFT JOIN alliances ON (players.alliance_id = alliances.alliance_id)
                WHERE islands.type = offsets.type
            "
        );
        for (index, constraint) in constraints.iter().enumerate() {
            statement_text += &format!(
                " AND {}.{} {} ?{}",
                constraint.constraint_type.table(),
                constraint.constraint_type.property(),
                constraint.comparator,
                index + 1
            );
        }
        if constraint_type.is_string() {
            statement_text += &*format!(" ORDER BY LOWER({ct_table}.{ct_property})");
        } else {
            statement_text += &*format!(" ORDER BY {ct_table}.{ct_property}");
        }

        // building a list of &dyn turned out to be very much non trivial.
        // we can't cast our stuff to &dyn in  a for loop, because the compiler
        // can't prove that we hold onto it for long enough, but we also can't
        // return early from the outer function in a map statement. so it's a bit
        // of both for now
        let mut query_parameters: Vec<EitherOr> = Vec::new();
        for constraint in constraints {
            if constraint.constraint_type.is_string() {
                query_parameters.push(EitherOr::A(constraint.value.clone()));
            } else {
                let opt_parsed = constraint.value.parse::<usize>();
                if let Ok(parsed) = opt_parsed {
                    query_parameters.push(EitherOr::B(parsed));
                } else {
                    return Ok(Vec::new());
                }
            }
        }

        let query_parameters: Vec<&dyn ToSql> = query_parameters
            .iter()
            .map(|param| param as &dyn ToSql)
            .collect();

        let mut statement = self
            .connection
            .prepare(&statement_text)
            .context("Failed to get names from database (build statement)")?;
        let rows = statement
            .query(query_parameters.as_slice())
            .context("Failed to get names from the database (perform query)")?
            .mapped(|row| {
                if constraint_type.is_string() {
                    row.get::<usize, String>(0)
                } else {
                    let value_option = row.get::<usize, usize>(0);
                    match value_option {
                        Ok(value) => Ok(format!("{value}")),
                        Err(err) => Err(err),
                    }
                }
            })
            .filter_map(Result::ok)
            .collect();

        Ok(rows)
    }

    pub fn get_towns_for_constraints(
        &self,
        constraints: &[Constraint],
    ) -> anyhow::Result<Vec<Town>> {
        if constraints.is_empty() {
            return Ok(Vec::new());
        }

        let mut statement_text = String::from(
            "SELECT towns.*, offsets.offset_x, offsets.offset_y, players.name, alliances.name from 
                towns 
                LEFT JOIN islands ON (towns.island_x = islands.x AND towns.island_y = islands.y)
                LEFT JOIN offsets ON (towns.slot_number = offsets.slot_number)
                LEFT JOIN players ON (towns.player_id = players.player_id)
                LEFT JOIN alliances ON (players.alliance_id = alliances.alliance_id)
                WHERE islands.type = offsets.type",
        );
        for (index, constraint) in constraints.iter().enumerate() {
            statement_text += &format!(
                " AND {}.{} {} ?{}",
                constraint.constraint_type.table(),
                constraint.constraint_type.property(),
                constraint.comparator,
                index + 1
            );
        }

        // building a list of &dyn turned out to be very much non trivial.
        // we can't cast our stuff to &dyn in  a for loop, because the compiler
        // can't prove that we hold onto it for long enough, but we also can't
        // return early from the outer function in a map statement. so it's a bit
        // of both for now
        let mut query_parameters: Vec<EitherOr> = Vec::new();
        for constraint in constraints {
            if constraint.constraint_type.is_string() {
                query_parameters.push(EitherOr::A(constraint.value.clone()));
            } else {
                let opt_parsed = constraint.value.parse::<usize>();
                if let Ok(parsed) = opt_parsed {
                    query_parameters.push(EitherOr::B(parsed));
                } else {
                    return Ok(Vec::new());
                }
            }
        }

        let query_parameters: Vec<&dyn ToSql> = query_parameters
            .iter()
            .map(|param| param as &dyn ToSql)
            .collect();

        let mut statement = self
            .connection
            .prepare(&statement_text)
            .context("Failed to get ghost towns from database (build statement)")?;
        let rows = statement
            .query(query_parameters.as_slice())
            .context("Failed to get ghost towns from the database (perform query)")?
            .mapped(Town::from)
            .collect::<std::result::Result<Vec<Town>, rusqlite::Error>>()
            .context("Failed to create a town from row")?;

        Ok(rows)
    }
}
