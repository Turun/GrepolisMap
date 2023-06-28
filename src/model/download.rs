/// this takes care of fetching the data and putting it into a database
use std::{future::Future, sync::mpsc};
use tokio;

use reqwest;
use rusqlite::{self, types::ToSqlOutput, ToSql};

use crate::message::{MessageToView, Progress};
use crate::towns::{Constraint, ConstraintType, Town};

use super::offset_data;

async fn download_generic<U>(client: &reqwest::Client, url: U) -> Result<String, reqwest::Error>
where
    U: reqwest::IntoUrl + std::fmt::Display,
{
    let url_text = format!("{url}");
    let result = client.get(url).send().await?;
    println!("Got status {} for url {}", result.status(), url_text);
    let text = result.text().await?;

    Ok(text)
}

fn make_client() -> reqwest::Client {
    reqwest::Client::builder()
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
    pub fn get_all_towns(&self) -> Vec<Town> {
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
            .expect("Failed to get towns from database (build statement)");
        let rows = statement
            .query([])
            .expect("Failed to get towns from the database (perform query)")
            .mapped(Town::from)
            .map(|town_option| town_option.expect("Failed to create a town from row"))
            .collect();

        rows
    }

    pub fn get_ghost_towns(&self) -> Vec<Town> {
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
            .expect("Failed to get ghost towns from database (build statement)");
        let rows = statement
            .query([])
            .expect("Failed to get ghost towns from the database (perform query)")
            .mapped(Town::from)
            .map(|town_option| town_option.expect("Failed to create a town from row"))
            .collect();

        rows
    }

    pub fn get_names_for_constraint_type(&self, constraint_type: &ConstraintType) -> Vec<String> {
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
            .expect("Failed to get names from database (build statement)");
        let rows = statement
            .query([])
            .expect("Failed to get names from the database (perform query)")
            .mapped(|row| {
                if constraint_type.is_string() {
                    let value_option = row.get::<usize, String>(0);
                    let value = value_option.expect("Failed to collect names from rows");
                    Ok(value)
                } else {
                    let value_option = row.get::<usize, usize>(0);
                    let value = value_option.expect("Failed to collect names from rows");
                    Ok(format!("{value}"))
                }
            })
            .map(Result::unwrap)
            .collect();

        rows
    }

    pub fn get_names_for_constraint_type_in_constraints(
        &self,
        constraint_type: &ConstraintType,
        constraints: &[&Constraint],
    ) -> Vec<String> {
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
                    return Vec::new();
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
            .expect("Failed to get names from database (build statement)");
        let rows = statement
            .query(query_parameters.as_slice())
            .expect("Failed to get names from the database (perform query)")
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

        rows
    }

    pub fn get_towns_for_constraints(&self, constraints: &[&Constraint]) -> Vec<Town> {
        if constraints.is_empty() {
            return Vec::new();
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
                    return Vec::new();
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
            .expect("Failed to get ghost towns from database (build statement)");
        let rows = statement
            .query(query_parameters.as_slice())
            .expect("Failed to get ghost towns from the database (perform query)")
            .mapped(Town::from)
            .map(|town_option| town_option.expect("Failed to create a town from row"))
            .collect();

        rows
    }

    pub fn create_for_world(
        server_id: &str,
        sender: mpsc::Sender<MessageToView>,
        ctx: &egui::Context,
    ) -> Result<Self, rusqlite::Error> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        runtime.block_on(async { Self::async_create_for_world(server_id, sender, ctx).await })
    }

    pub async fn async_create_for_world(
        server_id: &str,
        sender: mpsc::Sender<MessageToView>,
        ctx: &egui::Context,
    ) -> Result<Self, rusqlite::Error> {
        let reqwest_client = make_client();

        let mut conn =
            rusqlite::Connection::open_in_memory().expect("Failed to open in memory database");
        let data_players = download_generic(
            &reqwest_client,
            format!("https://{server_id}.grepolis.com/data/players.txt"),
        );
        let data_alliances = download_generic(
            &reqwest_client,
            format!("https://{server_id}.grepolis.com/data/alliances.txt"),
        );
        let data_towns = download_generic(
            &reqwest_client,
            format!("https://{server_id}.grepolis.com/data/towns.txt"),
        );
        let data_islands = download_generic(
            &reqwest_client,
            format!("https://{server_id}.grepolis.com/data/islands.txt"),
        );

        sender
            .send(MessageToView::Loading(Progress::Started))
            .expect("Failed to send progressupdate 1 to view");
        ctx.request_repaint();
        Database::create_table_offsets(&mut conn);
        sender
            .send(MessageToView::Loading(Progress::IslandOffsets))
            .expect("Failed to send progressupdate 2 to view");
        ctx.request_repaint();
        Database::create_table_alliances(&mut conn, data_alliances).await?;
        sender
            .send(MessageToView::Loading(Progress::Alliances))
            .expect("Failed to send progressupdate 3 to view");
        ctx.request_repaint();
        Database::create_table_players(&mut conn, data_players).await?;
        sender
            .send(MessageToView::Loading(Progress::Players))
            .expect("Failed to send progressupdate 4 to view");
        ctx.request_repaint();
        Database::create_table_towns(&mut conn, data_towns).await?;
        sender
            .send(MessageToView::Loading(Progress::Towns))
            .expect("Failed to send progressupdate 5 to view");
        ctx.request_repaint();
        Database::create_table_islands(&mut conn, data_islands).await?;
        sender
            .send(MessageToView::Loading(Progress::Islands))
            .expect("Failed to send progressupdate 6 to view");
        ctx.request_repaint();
        Ok(Self { connection: conn })
    }

    async fn create_table_players(
        connection: &mut rusqlite::Connection,
        data: impl Future<Output = Result<String, reqwest::Error>>,
    ) -> Result<(), rusqlite::Error> {
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
            .expect("Failed to create players table");

        let transaction = connection
            .transaction()
            .expect("Failed to start transaction for table creation players");

        let mut prepared_statement = transaction
            .prepare("INSERT INTO players VALUES(?1, ?2, ?3, ?4, ?5, ?6)")
            .expect("Failed to prepare statement for players");
        for line in data.await.expect("Failed to download player data").lines() {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values.next().expect(&format!("No player id in {line}")),
                    {
                        let text = values.next().expect(&format!("No player name in {line}"));
                        let decoded = form_urlencoded::parse(text.as_bytes())
                            .map(|(key, val)| [key, val].concat())
                            .collect::<String>();
                        decoded
                    },
                    {
                        let text = values.next().expect(&format!("No alliance id in {line}"));
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    },
                    values.next().expect(&format!("No player pts in {line}")),
                    values.next().expect(&format!("No player rank in {line}")),
                    values.next().expect(&format!("No player town in {line}")),
                ))
                .expect(&format!("Failed to insert into players from line {line}"));
        }
        drop(prepared_statement);
        transaction
            .commit()
            .expect("Failed to commit transaction for table players");
        Ok(())
    }

    async fn create_table_alliances(
        connection: &mut rusqlite::Connection,
        data: impl Future<Output = Result<String, reqwest::Error>>,
    ) -> Result<(), rusqlite::Error> {
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
            .expect("Failed to create table alliances");

        let transaction = connection
            .transaction()
            .expect("Failed to start transaction for table creation alliances");
        let mut prepared_statement =
            transaction.prepare("INSERT INTO alliances VALUES(?1, ?2, ?3, ?4, ?5, ?6)")?;
        for line in data
            .await
            .expect("Failed to download alliance data")
            .lines()
        {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values.next().expect(&format!("No ally id in {line}")),
                    {
                        let text = values.next().expect(&format!("No ally name in {line}"));
                        let decoded = form_urlencoded::parse(text.as_bytes())
                            .map(|(key, val)| [key, val].concat())
                            .collect::<String>();
                        decoded
                    },
                    values.next().expect(&format!("No ally pts in {line}")),
                    values.next().expect(&format!("No ally towns in {line}")),
                    values.next().expect(&format!("No ally membrs in {line}")),
                    values.next().expect(&format!("No ally rank in {line}")),
                ))
                .expect(&format!("Failed to insert into alliances for line {line}"));
        }
        drop(prepared_statement);
        transaction
            .commit()
            .expect("Failed to commit transaction for table alliances");
        Ok(())
    }

    async fn create_table_towns(
        connection: &mut rusqlite::Connection,
        data: impl Future<Output = Result<String, reqwest::Error>>,
    ) -> Result<(), rusqlite::Error> {
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
            .expect("Failed to create table towns");

        let transaction = connection
            .transaction()
            .expect("Failed to start transaction for table towns creation ");
        let mut prepared_statement = transaction
            .prepare("INSERT INTO towns VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .expect("Failed to prepare statement for towns");
        for line in data.await.expect("Failed to download town data").lines() {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values.next().expect(&format!("No town id in {line}")),
                    {
                        let text = values.next().expect(&format!("No player id in {line}"));
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    },
                    {
                        let text = values.next().expect(&format!("No town name in {line}"));
                        let decoded = form_urlencoded::parse(text.as_bytes())
                            .map(|(key, val)| [key, val].concat())
                            .collect::<String>();
                        decoded
                    },
                    values.next().expect(&format!("No town x in {line}")),
                    values.next().expect(&format!("No town y pts in {line}")),
                    values.next().expect(&format!("No town slotnr in {line}")),
                    values.next().expect(&format!("No town points in {line}")),
                ))
                .expect(&format!("Failed to insert into towns from line {line}"));
        }
        drop(prepared_statement);
        transaction
            .commit()
            .expect("Failed to commit transaction for table towns");
        Ok(())
    }

    async fn create_table_islands(
        connection: &mut rusqlite::Connection,
        data: impl Future<Output = Result<String, reqwest::Error>>,
    ) -> Result<(), rusqlite::Error> {
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
            .expect("Failed to create table islands");
        let transaction = connection
            .transaction()
            .expect("Failed to start transaction for table creation islands");
        let mut prepared_statement = transaction
            .prepare("INSERT INTO islands VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)")
            .expect("Failed to prepare statement for islands");
        for line in data.await.expect("Failed to download island data").lines() {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values.next().expect(&format!("No island id in {line}")),
                    values.next().expect(&format!("No island x in {line}")),
                    values.next().expect(&format!("No island y in {line}")),
                    values.next().expect(&format!("No island type in {line}")),
                    values.next().expect(&format!("No island town in {line}")),
                    values.next().expect(&format!("No island more in {line}")),
                    values.next().expect(&format!("No island less in {line}")),
                ))
                .expect(&format!("Failed to insert into islands from line {line}"));
        }
        drop(prepared_statement);
        transaction
            .commit()
            .expect("Failed to commit transaction for table islands");
        Ok(())
    }
    fn create_table_offsets(connection: &mut rusqlite::Connection) {
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
            .expect("Failed to create table offsets");
        let transaction = connection
            .transaction()
            .expect("Failed to start transaction for table creation offsets");
        let mut prepared_statement = transaction
            .prepare("INSERT INTO offsets VALUES(?1, ?2, ?3, ?4)")
            .expect("Failed to prepare statement for offsets");
        for line in offset_data::OFFSET_DATA.lines() {
            let mut values = line.split(',');
            prepared_statement
                .execute((
                    values.next().expect(&format!("No offset tyep in {line}")),
                    values.next().expect(&format!("No offset x in {line}")),
                    values.next().expect(&format!("No offset y in {line}")),
                    values.next().expect(&format!("No offset slot in {line}")),
                ))
                .expect(&format!("Failed to insert into offsets from line {line}"));
        }
        drop(prepared_statement);
        transaction
            .commit()
            .expect("Failed to commit transaction for table offsets");
    }
}
