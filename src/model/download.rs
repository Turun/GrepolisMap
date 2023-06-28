/// this takes care of fetching the data and putting it into a database
use std::future::Future;

use reqwest;
use rusqlite;

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
    pub async fn create_for_world(server_id: &str) -> Result<Self, rusqlite::Error> {
        let reqwest_client = make_client();

        let conn =
            rusqlite::Connection::open_in_memory().expect("Failed to open in memory database");
        let data_players = download_generic(
            &reqwest_client,
            format!("https://{}.grepolis.com/data/players.txt.gz", server_id),
        );
        let data_alliances = download_generic(
            &reqwest_client,
            format!("https://{}.grepolis.com/data/alliances.txt.gz", server_id),
        );
        let data_towns = download_generic(
            &reqwest_client,
            format!("https://{}.grepolis.com/data/towns.txt.gz", server_id),
        );
        let data_islands = download_generic(
            &reqwest_client,
            format!("https://{}.grepolis.com/data/islands.txt.gz", server_id),
        );

        Database::create_table_players(&conn, data_players).await?;
        Database::create_table_alliances(&conn, data_alliances).await?;
        Database::create_table_towns(&conn, data_towns).await?;
        Database::create_table_islands(&conn, data_islands).await?;

        Ok(Self { connection: conn })
    }

    async fn create_table_players(
        connection: &rusqlite::Connection,
        data: impl Future<Output = Result<String, reqwest::Error>>,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "CREATE TABLE players(
            player_id INTEGER UNIQUE PRIMARY KEY, 
            name TEXT UNIQUE, 
            alliance_id INTEGER, 
            points INTEGER, 
            rank INTEGER, 
            towns INTEGER, 
            FOREIGN KEY(alliance_id) REFERENCES alliances(alliance_id))",
            (),
        )?;
        let mut prepared_statement =
            connection.prepare("INSERT INTO players VALUES(?1, ?2, ?3, ?4, ?5, ?6)")?;
        for line in data.await.expect("Failed to download player data").lines() {
            let mut values = line.split(",");
            prepared_statement.execute((
                values.next().expect(&format!("No player id in {}", line)),
                values.next().expect(&format!("No player name in {}", line)),
                values.next().expect(&format!("No alliance id in {}", line)),
                values.next().expect(&format!("No player pts in {}", line)),
                values.next().expect(&format!("No player rank in {}", line)),
                values.next().expect(&format!("No player town in {}", line)),
            ))?;
        }
        Ok(())
    }

    async fn create_table_alliances(
        connection: &rusqlite::Connection,
        data: impl Future<Output = Result<String, reqwest::Error>>,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "CREATE TABLE alliances(
            alliance_id INTEGER UNIQUE PRIMARY KEY, 
            name TEXT UNIQUE, 
            points INTEGER,
            towns INTEGER,
            members INTEGER,
            rank INTEGER",
            (),
        )?;
        let mut prepared_statement =
            connection.prepare("INSERT INTO alliances VALUES(?1, ?2, ?3, ?4, ?5, ?6)")?;
        for line in data
            .await
            .expect("Failed to download alliance data")
            .lines()
        {
            let mut values = line.split(",");
            prepared_statement.execute((
                values.next().expect(&format!("No ally id in {}", line)),
                values.next().expect(&format!("No ally name in {}", line)),
                values.next().expect(&format!("No ally pts in {}", line)),
                values.next().expect(&format!("No ally towns in {}", line)),
                values.next().expect(&format!("No ally membrs in {}", line)),
                values.next().expect(&format!("No ally rank in {}", line)),
            ))?;
        }
        Ok(())
    }

    async fn create_table_towns(
        connection: &rusqlite::Connection,
        data: impl Future<Output = Result<String, reqwest::Error>>,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
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
        )?;
        let mut prepared_statement =
            connection.prepare("INSERT INTO towns VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;
        for line in data.await.expect("Failed to download town data").lines() {
            let mut values = line.split(",");
            prepared_statement.execute((
                values.next().expect(&format!("No town id in {}", line)),
                values.next().expect(&format!("No player id in {}", line)),
                values.next().expect(&format!("No town name in {}", line)),
                values.next().expect(&format!("No town x in {}", line)),
                values.next().expect(&format!("No town y pts in {}", line)),
                values.next().expect(&format!("No town slotnr in {}", line)),
                values.next().expect(&format!("No town points in {}", line)),
            ))?;
        }
        Ok(())
    }

    async fn create_table_islands(
        connection: &rusqlite::Connection,
        data: impl Future<Output = Result<String, reqwest::Error>>,
    ) -> Result<(), rusqlite::Error> {
        connection.execute(
            "CREATE TABLE islands(
            island_id INTEGER UNIQUE PRIMARY KEY, 
            x INTEGER, 
            y INTEGER, 
            type INTEGER, 
            num_towns INTEGER, 
            ressource_plus TEXT, 
            ressource_minus TEXT)",
            (),
        )?;
        let mut prepared_statement =
            connection.prepare("INSERT INTO islands VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;
        for line in data.await.expect("Failed to download island data").lines() {
            let mut values = line.split(",");
            prepared_statement.execute((
                values.next().expect(&format!("No island id in {}", line)),
                values.next().expect(&format!("No island x in {}", line)),
                values.next().expect(&format!("No island y in {}", line)),
                values.next().expect(&format!("No island type in {}", line)),
                values.next().expect(&format!("No island town in {}", line)),
                values.next().expect(&format!("No island more in {}", line)),
                values.next().expect(&format!("No island less in {}", line)),
            ))?;
        }
        Ok(())
    }
}
