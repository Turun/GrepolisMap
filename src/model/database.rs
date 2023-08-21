use anyhow::{Context, Result};
use rusqlite::{Connection, Statement};

use crate::constraint::Comparator;
use crate::model::Constraint;
use crate::model::ConstraintType;
use crate::town::Town;

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

impl ToSqlFragment for Constraint {
    fn to_sql_fragment(&self, parameter_index: usize) -> String {
        format!(
            "{}.{} {} ?{}",
            self.constraint_type.table(),
            self.constraint_type.property(),
            self.comparator,
            parameter_index + 1
        )
    }
}

static TOWN_SELECTION: &str =
    "towns.*, offsets.offset_x, offsets.offset_y, players.name, alliances.name";

impl Database {
    fn prepare_statement<'a, SQL>(
        connection: &'a Connection,
        selection_clause: &str,
        filter_clauses: &[SQL],
        order_clause: Option<&str>,
    ) -> anyhow::Result<Statement<'a>>
    where
        SQL: ToSqlFragment,
    {
        let mut sql_fragments = filter_clauses
            .iter()
            .enumerate()
            .map(|(i, x)| x.to_sql_fragment(i))
            .collect();
        let mut sql_start = vec![format!(
            "SELECT {selection_clause} from \n\
                towns \n\
                LEFT JOIN islands ON (towns.island_x = islands.x AND towns.island_y = islands.y) \n\
                LEFT JOIN offsets ON (towns.slot_number = offsets.slot_number) \n\
                LEFT JOIN players ON (towns.player_id = players.player_id) \n\
                LEFT JOIN alliances ON (players.alliance_id = alliances.alliance_id) \n\
                WHERE islands.type = offsets.type",
        )];
        sql_start.append(&mut sql_fragments);
        let mut sql_text = sql_start.join(" \nAND ");
        if let Some(text) = order_clause {
            sql_text += " \n";
            sql_text += text;
        }

        let statement = connection
            .prepare(&sql_text)
            .context("Failed to get towns from database (build statement)")?;
        Ok(statement)
    }

    fn bind_statement(
        prepared_statement: &mut Statement<'_>,
        constraints: &[Constraint],
    ) -> anyhow::Result<()> {
        for (index, constraint) in constraints.iter().enumerate() {
            // TODO implement in selction constraint value
            prepared_statement.raw_bind_parameter(index + 1, &constraint.value)?;
        }
        Ok(())
    }

    pub fn get_all_towns(&self) -> anyhow::Result<Vec<Town>> {
        let mut statement =
            Self::prepare_statement::<AllTowns>(&self.connection, TOWN_SELECTION, &[], None)?;
        let rows = statement
            .query([])
            .context("Failed to get towns from the database (perform query)")?
            .mapped(Town::from)
            .collect::<std::result::Result<Vec<Town>, rusqlite::Error>>()
            .context("Failed to create a town from row")?;

        Ok(rows)
    }

    pub fn get_ghost_towns(&self) -> anyhow::Result<Vec<Town>> {
        let mut statement =
            Self::prepare_statement(&self.connection, TOWN_SELECTION, &[GhostTown], None)?;
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
        let order_clause = if constraint_type.is_string() {
            format!("ORDER BY LOWER({ct_table}.{ct_property})")
        } else {
            format!("ORDER BY {ct_table}.{ct_property}")
        };
        let mut statement = Self::prepare_statement(
            &self.connection,
            &format!("DISTINCT {ct_table}.{ct_property}"),
            constraints,
            Some(&order_clause),
        )?;

        Self::bind_statement(&mut statement, constraints)?;

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
        constraints: &[Constraint],
    ) -> anyhow::Result<Vec<Town>> {
        if constraints.is_empty() {
            return Ok(Vec::new());
        }

        let mut statement =
            Self::prepare_statement(&self.connection, TOWN_SELECTION, constraints, None)?;

        Self::bind_statement(&mut statement, constraints)?;

        let rows = statement
            .raw_query()
            .mapped(Town::from)
            .collect::<std::result::Result<Vec<Town>, rusqlite::Error>>()
            .context("Failed to create a town from row")?;

        Ok(rows)
    }
}
