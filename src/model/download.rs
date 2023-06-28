use egui::FontData;
/// this takes care of fetching the data and putting it into a database
use std::future::Future;
use tokio;

use reqwest;
use rusqlite;

use crate::message::Town;

async fn download_generic<U>(client: &reqwest::Client, url: U) -> Result<String, reqwest::Error>
where
    U: reqwest::IntoUrl + std::fmt::Display,
{
    let url_text = format!("{}", url);
    let result = client.get(url).send().await?;
    println!("Got status {} for url {}", result.status(), url_text);
    let text = result.text().await?;
    return Ok(text);
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

impl Database {
    pub fn get_all_towns(&self) -> Vec<Town> {
        let mut statement = self
            .connection
            .prepare("SELECT * from towns")
            .expect("Failed to get towns from database (build statement)");
        let rows = statement
            .query([])
            .expect("Failed to get towns from the database (perform query)")
            .mapped(|row| Town::from(row))
            .map(|town_option| town_option.expect("Failed to create a town from row"))
            .collect();
        return rows;
    }

    pub fn get_ghost_towns(&self) -> Vec<Town> {
        let mut statement = self
            .connection
            .prepare("SELECT * from towns WHERE player_id=\"\"")
            .expect("Failed to get ghost towns from database (build statement)");
        let rows = statement
            .query([])
            .expect("Failed to get ghost towns from the database (perform query)")
            .mapped(|row| Town::from(row))
            .map(|town_option| town_option.expect("Failed to create a town from row"))
            .collect();
        return rows;
    }

    pub fn create_for_world(server_id: &str) -> Result<Self, rusqlite::Error> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        return runtime.block_on(async { Self::async_create_for_world(server_id).await });
    }

    pub async fn async_create_for_world(server_id: &str) -> Result<Self, rusqlite::Error> {
        let reqwest_client = make_client();

        let mut conn =
            rusqlite::Connection::open_in_memory().expect("Failed to open in memory database");
        let data_players = download_generic(
            &reqwest_client,
            format!("https://{}.grepolis.com/data/players.txt", server_id),
        );
        let data_alliances = download_generic(
            &reqwest_client,
            format!("https://{}.grepolis.com/data/alliances.txt", server_id),
        );
        let data_towns = download_generic(
            &reqwest_client,
            format!("https://{}.grepolis.com/data/towns.txt", server_id),
        );
        let data_islands = download_generic(
            &reqwest_client,
            format!("https://{}.grepolis.com/data/islands.txt", server_id),
        );

        Database::create_table_alliances(&mut conn, data_alliances).await?;
        Database::create_table_players(&mut conn, data_players).await?;
        Database::create_table_towns(&mut conn, data_towns).await?;
        Database::create_table_islands(&mut conn, data_islands).await?;
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
            FOREIGN KEY(alliance_id) REFERENCES alliances(alliance_id))",
                (),
            )
            .expect("Failed to create players table");

        let transaction = connection
            .transaction()
            .expect("Failed to start transaction for table creation players");

        let mut prepared_statement = (&transaction)
            .prepare("INSERT INTO players VALUES(?1, ?2, ?3, ?4, ?5, ?6)")
            .expect("Failed to prepare statement for players");
        for line in data.await.expect("Failed to download player data").lines() {
            let mut values = line.split(",");
            prepared_statement
                .execute((
                    values.next().expect(&format!("No player id in {}", line)),
                    values.next().expect(&format!("No player name in {}", line)),
                    {
                        let text = values.next().expect(&format!("No alliance id in {}", line));
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    },
                    values.next().expect(&format!("No player pts in {}", line)),
                    values.next().expect(&format!("No player rank in {}", line)),
                    values.next().expect(&format!("No player town in {}", line)),
                ))
                .expect(&format!("Failed to insert into players from line {}", line));
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
            let mut values = line.split(",");
            prepared_statement
                .execute((
                    values.next().expect(&format!("No ally id in {}", line)),
                    values.next().expect(&format!("No ally name in {}", line)),
                    values.next().expect(&format!("No ally pts in {}", line)),
                    values.next().expect(&format!("No ally towns in {}", line)),
                    values.next().expect(&format!("No ally membrs in {}", line)),
                    values.next().expect(&format!("No ally rank in {}", line)),
                ))
                .expect(&format!(
                    "Failed to insert into alliances for line {}",
                    line
                ));
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
            FOREIGN KEY(player_id) REFERENCES players(player_id))",
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
            let mut values = line.split(",");
            prepared_statement
                .execute((
                    values.next().expect(&format!("No town id in {}", line)),
                    {
                        let text = values.next().expect(&format!("No player id in {}", line));
                        if text.is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    },
                    values.next().expect(&format!("No town name in {}", line)),
                    values.next().expect(&format!("No town x in {}", line)),
                    values.next().expect(&format!("No town y pts in {}", line)),
                    values.next().expect(&format!("No town slotnr in {}", line)),
                    values.next().expect(&format!("No town points in {}", line)),
                ))
                .expect(&format!("Failed to insert into towns from line {}", line));
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
            num_towns INTEGER, 
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
            let mut values = line.split(",");
            prepared_statement
                .execute((
                    values.next().expect(&format!("No island id in {}", line)),
                    values.next().expect(&format!("No island x in {}", line)),
                    values.next().expect(&format!("No island y in {}", line)),
                    values.next().expect(&format!("No island type in {}", line)),
                    values.next().expect(&format!("No island town in {}", line)),
                    values.next().expect(&format!("No island more in {}", line)),
                    values.next().expect(&format!("No island less in {}", line)),
                ))
                .expect(&format!("Failed to insert into islands from line {}", line));
        }
        drop(prepared_statement);
        transaction
            .commit()
            .expect("Failed to commit transaction for table islands");
        Ok(())
    }
}
