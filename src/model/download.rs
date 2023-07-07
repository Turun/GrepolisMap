/// this takes care of fetching the data and putting it into a database
use std::sync::mpsc;

use anyhow::{Context, Result};
use reqwest;
use rusqlite::{self, types::ToSqlOutput, ToSql};

use crate::message::{MessageToView, Progress};
use crate::towns::{Constraint, ConstraintType, Town};

use super::offset_data;

fn download_generic<U>(
    client: &reqwest::blocking::Client,
    url: U,
) -> std::result::Result<String, reqwest::Error>
where
    U: reqwest::IntoUrl + std::fmt::Display,
{
    let url_text = format!("{url}");
    let result = client.get(url).send()?;
    println!("Got status {} for url {}", result.status(), url_text);
    let text = result.text()?;

    Ok(text)
}

fn make_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("Rust Grepolis Map - Turun")
        .gzip(true)
        .deflate(true)
        .build()
        .unwrap()
}

pub struct Database {
    connection: rusqlite::Connection,
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

    pub fn create_for_world(
        server_id: &str,
        sender: &mpsc::Sender<MessageToView>,
        ctx: &egui::Context,
    ) -> anyhow::Result<Self> {
        let reqwest_client = make_client();

        let mut conn =
            rusqlite::Connection::open_in_memory().context("Failed to open in memory database")?;
        let thread_client = reqwest_client.clone();
        let thread_server_id = String::from(server_id);
        let handle_data_players = std::thread::spawn(move || {
            download_generic(
                &thread_client,
                format!("https://{thread_server_id}.grepolis.com/data/players.txt"),
            )
        });
        let thread_client = reqwest_client.clone();
        let thread_server_id = String::from(server_id);
        let handle_data_alliances = std::thread::spawn(move || {
            download_generic(
                &thread_client,
                format!("https://{thread_server_id}.grepolis.com/data/alliances.txt"),
            )
        });
        let thread_client = reqwest_client.clone();
        let thread_server_id = String::from(server_id);
        let handle_data_towns = std::thread::spawn(move || {
            download_generic(
                &thread_client,
                format!("https://{thread_server_id}.grepolis.com/data/towns.txt"),
            )
        });
        let thread_client = reqwest_client;
        let thread_server_id = String::from(server_id);
        let handle_data_islands = std::thread::spawn(move || {
            download_generic(
                &thread_client,
                format!("https://{thread_server_id}.grepolis.com/data/islands.txt"),
            )
        });

        sender
            .send(MessageToView::Loading(Progress::Started))
            .context("Failed to send progressupdate 1 to view")?;
        ctx.request_repaint();

        Database::create_table_offsets(&mut conn)?;
        sender
            .send(MessageToView::Loading(Progress::IslandOffsets))
            .context("Failed to send progressupdate 2 to view")?;
        ctx.request_repaint();

        let data_alliances = handle_data_alliances
            .join()
            .expect("Failed to join AllianceData fetching thread");
        Database::create_table_alliances(&mut conn, data_alliances)?;
        sender
            .send(MessageToView::Loading(Progress::Alliances))
            .context("Failed to send progressupdate 3 to view")?;
        ctx.request_repaint();

        let data_players = handle_data_players
            .join()
            .expect("Failed to join PlayerData fetching thread");
        Database::create_table_players(&mut conn, data_players)?;
        sender
            .send(MessageToView::Loading(Progress::Players))
            .context("Failed to send progressupdate 4 to view")?;
        ctx.request_repaint();

        let data_towns = handle_data_towns
            .join()
            .expect("Failed to join AllianceData fetching thread");
        Database::create_table_towns(&mut conn, data_towns)?;
        sender
            .send(MessageToView::Loading(Progress::Towns))
            .context("Failed to send progressupdate 5 to view")?;
        ctx.request_repaint();

        let data_islands = handle_data_islands
            .join()
            .expect("Failed to join AllianceData fetching thread");
        Database::create_table_islands(&mut conn, data_islands)?;
        sender
            .send(MessageToView::Loading(Progress::Islands))
            .context("Failed to send progressupdate 6 to view")?;
        ctx.request_repaint();
        Ok(Self { connection: conn })
    }

    fn create_table_players(
        connection: &mut rusqlite::Connection,
        data: Result<String, reqwest::Error>,
    ) -> anyhow::Result<()> {
        connection
            .execute(
                "CREATE TABLE players(
                player_id INTEGER UNIQUE PRIMARY KEY, 
                name TEXT UNIQUE, 
                alliance_id INTEGER, 
                points INTEGER, 
                rank INTEGER, 
                towns INTEGER, 
                FOREIGN KEY(alliance_id) REFERENCES alliances(alliance_id) DEFERRABLE)",
                (),
            )
            .context("Failed to create players table")?;

        let transaction = connection
            .transaction()
            .context("Failed to start transaction for table creation players")?;

        let mut prepared_statement = transaction
            .prepare("INSERT INTO players VALUES(?1, ?2, ?3, ?4, ?5, ?6)")
            .context("Failed to prepare statement for players")?;
        for line in data.context("Failed to download player data")?.lines() {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values
                        .next()
                        .with_context(|| format!("No player id in {line}"))?,
                    {
                        let text = values
                            .next()
                            .with_context(|| format!("No player name in {line}"))?;
                        let decoded = form_urlencoded::parse(text.as_bytes())
                            .map(|(key, val)| [key, val].concat())
                            .collect::<String>();
                        decoded
                    },
                    {
                        let text = values
                            .next()
                            .with_context(|| format!("No alliance id in {line}"))?;
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    },
                    values
                        .next()
                        .with_context(|| format!("No player pts in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No player rank in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No player town in {line}"))?,
                ))
                .with_context(|| format!("Failed to insert into players from line {line}"))?;
        }
        drop(prepared_statement);
        transaction
            .commit()
            .context("Failed to commit transaction for table players")?;
        Ok(())
    }

    fn create_table_alliances(
        connection: &mut rusqlite::Connection,
        data: Result<String, reqwest::Error>,
    ) -> anyhow::Result<()> {
        connection
            .execute(
                "CREATE TABLE alliances(
                alliance_id INTEGER UNIQUE PRIMARY KEY, 
                name TEXT UNIQUE, 
                points INTEGER,
                towns INTEGER,
                members INTEGER,
                rank INTEGER)",
                (),
            )
            .context("Failed to create table alliances")?;

        let transaction = connection
            .transaction()
            .context("Failed to start transaction for table creation alliances")?;
        let mut prepared_statement = transaction
            .prepare("INSERT INTO alliances VALUES(?1, ?2, ?3, ?4, ?5, ?6)")
            .context("Failed to prepare statement for alliances")?;
        for line in data.context("Failed to download alliance data")?.lines() {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values
                        .next()
                        .with_context(|| format!("No ally id in {line}"))?,
                    {
                        let text = values
                            .next()
                            .with_context(|| format!("No ally name in {line}"))?;
                        let decoded = form_urlencoded::parse(text.as_bytes())
                            .map(|(key, val)| [key, val].concat())
                            .collect::<String>();
                        decoded
                    },
                    values
                        .next()
                        .with_context(|| format!("No ally pts in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No ally towns in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No ally membrs in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No ally rank in {line}"))?,
                ))
                .with_context(|| format!("Failed to insert into alliances for line {line}"))?;
        }
        drop(prepared_statement);
        transaction
            .commit()
            .context("Failed to commit transaction for table alliances")?;
        Ok(())
    }

    fn create_table_towns(
        connection: &mut rusqlite::Connection,
        data: Result<String, reqwest::Error>,
    ) -> anyhow::Result<()> {
        connection
            .execute(
                "CREATE TABLE towns(
                town_id INTEGER UNIQUE PRIMARY KEY, 
                player_id INTEGER, 
                name TEXT, 
                island_x INTEGER, 
                island_y INTEGER, 
                slot_number INTEGER, 
                points INTEGER, 
                FOREIGN KEY(player_id) REFERENCES players(player_id) DEFERRABLE)",
                (),
            )
            .context("Failed to create table towns")?;

        let transaction = connection
            .transaction()
            .context("Failed to start transaction for table towns creation ")?;
        let mut prepared_statement = transaction
            .prepare("INSERT INTO towns VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .context("Failed to prepare statement for towns")?;
        for line in data.context("Failed to download town data")?.lines() {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values
                        .next()
                        .with_context(|| format!("No town id in {line}"))?,
                    {
                        let text = values
                            .next()
                            .with_context(|| format!("No player id in {line}"))?;
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    },
                    {
                        let text = values
                            .next()
                            .with_context(|| format!("No town name in {line}"))?;
                        let decoded = form_urlencoded::parse(text.as_bytes())
                            .map(|(key, val)| [key, val].concat())
                            .collect::<String>();
                        decoded
                    },
                    values
                        .next()
                        .with_context(|| format!("No town x in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No town y pts in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No town slotnr in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No town points in {line}"))?,
                ))
                .with_context(|| format!("Failed to insert into towns from line {line}"))?;
        }
        drop(prepared_statement);
        transaction
            .commit()
            .context("Failed to commit transaction for table towns")?;
        Ok(())
    }

    fn create_table_islands(
        connection: &mut rusqlite::Connection,
        data: Result<String, reqwest::Error>,
    ) -> anyhow::Result<()> {
        connection
            .execute(
                "CREATE TABLE islands(
                island_id INTEGER UNIQUE PRIMARY KEY, 
                x INTEGER, 
                y INTEGER, 
                type INTEGER, 
                towns INTEGER, 
                ressource_plus TEXT, 
                ressource_minus TEXT)",
                (),
            )
            .context("Failed to create table islands")?;
        let transaction = connection
            .transaction()
            .context("Failed to start transaction for table creation islands")?;
        let mut prepared_statement = transaction
            .prepare("INSERT INTO islands VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .context("Failed to prepare statement for islands")?;
        for line in data.context("Failed to download island data")?.lines() {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values
                        .next()
                        .with_context(|| format!("No island id in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No island x in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No island y in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No island type in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No island town in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No island more in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No island less in {line}"))?,
                ))
                .with_context(|| format!("Failed to insert into islands from line {line}"))?;
        }
        drop(prepared_statement);
        transaction
            .commit()
            .context("Failed to commit transaction for table islands")?;
        Ok(())
    }

    fn create_table_offsets(connection: &mut rusqlite::Connection) -> anyhow::Result<()> {
        connection
            .execute(
                "CREATE TABLE offsets(
                type INTEGER NOT NULL, 
                offset_x INTEGER NOT NULL, 
                offset_y INTEGER NOT NULL, 
                slot_number INTEGER NOT NULL,
                PRIMARY KEY (type, slot_number))",
                (),
            )
            .context("Failed to create table offsets")?;
        let transaction = connection
            .transaction()
            .context("Failed to start transaction for table creation offsets")?;
        let mut prepared_statement = transaction
            .prepare("INSERT INTO offsets VALUES(?1, ?2, ?3, ?4)")
            .context("Failed to prepare statement for offsets")?;
        for line in offset_data::OFFSET_DATA.lines() {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values
                        .next()
                        .with_context(|| format!("No offset tyep in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No offset x in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No offset y in {line}"))?,
                    values
                        .next()
                        .with_context(|| format!("No offset slot in {line}"))?,
                ))
                .with_context(|| format!("Failed to insert into offsets from line {line}"))?;
        }
        drop(prepared_statement);
        transaction
            .commit()
            .context("Failed to commit transaction for table offsets")?;
        Ok(())
    }
}
