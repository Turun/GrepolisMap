use super::database::{Alliance, BackendTown, DataTable, Database, Island, Offset, Player};
use super::offset_data;
use crate::message::{MessageToView, Progress};
use anyhow::{Context, Result};
use reqwest;
use std::path::Path;
use std::rc::Rc;
use std::sync::mpsc;

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

impl DataTable {
    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        todo!();
    }

    pub fn create_for_world(
        server_id: &str,
        filename: Option<&Path>,
        sender: &mpsc::Sender<MessageToView>,
        ctx: &egui::Context,
    ) -> anyhow::Result<Self> {
        let reqwest_client = make_client();

        if let Some(path) = filename {
            // TODO: load from file and return immediately
        };

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

        let offsets = Self::make_offsets();
        sender
            .send(MessageToView::Loading(Progress::IslandOffsets))
            .context("Failed to send progressupdate 2 to view")?;
        ctx.request_repaint();

        let data_alliances = handle_data_alliances
            .join()
            .expect("Failed to join AllianceData fetching thread")
            .context("Failed to download alliance data")?;
        let alliances = Self::parse_alliances(data_alliances)?;
        sender
            .send(MessageToView::Loading(Progress::Alliances))
            .context("Failed to send progressupdate 3 to view")?;
        ctx.request_repaint();

        let data_islands = handle_data_islands
            .join()
            .expect("Failed to join islandData fetching thread")
            .context("Failed to download island data")?;
        let islands = Self::parse_islands(data_islands)?;
        sender
            .send(MessageToView::Loading(Progress::Islands))
            .context("Failed to send progressupdate 3 to view")?;
        ctx.request_repaint();

        let data_players = handle_data_players
            .join()
            .expect("Failed to join PlayerData fetching thread")
            .context("Failed to download player data")?;
        let players = Self::parse_players(data_players, &alliances)?;
        sender
            .send(MessageToView::Loading(Progress::Players))
            .context("Failed to send progressupdate 3 to view")?;
        ctx.request_repaint();

        let data_towns = handle_data_towns
            .join()
            .expect("Failed to join TownData fetching thread")
            .context("Failed to download town data")?;
        let towns = Self::parse_towns(data_towns, &players, &islands, &offsets)?;
        sender
            .send(MessageToView::Loading(Progress::Towns))
            .context("Failed to send progressupdate 3 to view")?;
        ctx.request_repaint();

        Ok(Self { towns })
    }

    fn make_offsets() -> Vec<Rc<Offset>> {
        let lines: Vec<&str> = offset_data::OFFSET_DATA.lines().collect();
        let mut re = Vec::with_capacity(lines.len());
        for line in lines {
            let mut values = line.split(",");
            let typ: u8 = values.next().unwrap().parse().unwrap();
            let x: u16 = values.next().unwrap().parse().unwrap();
            let y: u16 = values.next().unwrap().parse().unwrap();
            let slot_number: u8 = values.next().unwrap().parse().unwrap();
            re.push(Rc::new(Offset {
                typ,
                x,
                y,
                slot_number,
            }))
        }
        return re;
    }

    fn parse_alliances(data: String) -> anyhow::Result<Vec<Rc<Alliance>>> {
        let lines: Vec<&str> = data.lines().collect();
        let mut re = Vec::with_capacity(lines.len());
        for line in lines {
            let mut values = line.split(',');

            let id = values
                .next()
                .with_context(|| format!("No ally id in {line}"))?
                .parse()
                .with_context(|| format!("No ally id in {line} that can be parsed as int"))?;
            let name = {
                let text = values
                    .next()
                    .with_context(|| format!("No ally name in {line}"))?;
                let decoded = form_urlencoded::parse(text.as_bytes())
                    .map(|(key, val)| [key, val].concat())
                    .collect::<String>();
                decoded
            };
            let points = values
                .next()
                .with_context(|| format!("No ally pts in {line}"))?
                .parse()
                .with_context(|| format!("No ally points in {line} that can be parsed as int"))?;
            let towns = values
                .next()
                .with_context(|| format!("No ally towns in {line}"))?
                .parse()
                .with_context(|| format!("No ally towns in {line} that can be parsed as int"))?;
            let members = values
                .next()
                .with_context(|| format!("No ally membrs in {line}"))?
                .parse()
                .with_context(|| format!("No ally members in {line} that can be parsed as int"))?;
            let rank = values
                .next()
                .with_context(|| format!("No ally rank in {line}"))?
                .parse()
                .with_context(|| format!("No ally rank in {line} that can be parsed as int"))?;
            re.push(Rc::new(Alliance {
                id,
                name,
                points,
                towns,
                members,
                rank,
            }))
        }
        return Ok(re);
    }

    fn parse_islands(data: String) -> anyhow::Result<Vec<Rc<Island>>> {
        let lines: Vec<&str> = data.lines().collect();
        let mut re = Vec::with_capacity(lines.len());
        for line in lines {
            let mut values = line.split(',');

            let id = values
                .next()
                .with_context(|| format!("No island id in {line}"))?
                .parse()
                .with_context(|| format!("No island id in {line} that can be parsed as int"))?;
            let x = values
                .next()
                .with_context(|| format!("No island x in {line}"))?
                .parse()
                .with_context(|| format!("No island x in {line} that can be parsed as int"))?;
            let y = values
                .next()
                .with_context(|| format!("No island y in {line}"))?
                .parse()
                .with_context(|| format!("No island y in {line} that can be parsed as int"))?;
            let typ = values
                .next()
                .with_context(|| format!("No island type in {line}"))?
                .parse()
                .with_context(|| format!("No island type in {line} that can be parsed as int"))?;
            let towns = values
                .next()
                .with_context(|| format!("No island towns in {line}"))?
                .parse()
                .with_context(|| format!("No island towns in {line} that can be parsed as int"))?;
            let ressource_plus = values
                .next()
                .with_context(|| format!("No island res+ in {line}"))?
                .to_string();
            let ressource_minus = values
                .next()
                .with_context(|| format!("No island res- in {line}"))?
                .to_string();
            re.push(Rc::new(Island {
                id,
                x,
                y,
                typ,
                towns,
                ressource_plus,
                ressource_minus,
            }))
        }
        return Ok(re);
    }

    fn parse_players(data: String, alliances: &[Rc<Alliance>]) -> anyhow::Result<Vec<Rc<Player>>> {
        let lines: Vec<&str> = data.lines().collect();
        let mut re = Vec::with_capacity(lines.len());
        for line in lines {
            let mut values = line.split(',');

            let id = values
                .next()
                .with_context(|| format!("No player id in {line}"))?
                .parse()
                .with_context(|| format!("No player id in {line} that can be parsed as int"))?;
            let name = {
                let text = values
                    .next()
                    .with_context(|| format!("No player name in {line}"))?;
                let decoded = form_urlencoded::parse(text.as_bytes())
                    .map(|(key, val)| [key, val].concat())
                    .collect::<String>();
                decoded
            };
            let opt_alliance_id = {
                let text = values
                    .next()
                    .with_context(|| format!("No player alliance id in {line}"))?;
                if text.is_empty() {
                    None
                } else {
                    Some(text.parse().with_context(|| {
                        format!("No player alliance id in {line} that can be parsed as int")
                    })?)
                }
            };
            let points = values
                .next()
                .with_context(|| format!("No player points in {line}"))?
                .parse()
                .with_context(|| format!("No player point in {line} that can be parsed as int"))?;
            let rank = values
                .next()
                .with_context(|| format!("No player rank in {line}"))?
                .parse()
                .with_context(|| format!("No player rank in {line} that can be parsed as int"))?;
            let towns = values
                .next()
                .with_context(|| format!("No player towns in {line}"))?
                .parse()
                .with_context(|| format!("No player towns in {line} that can be parsed as int"))?;

            let alliance_tuple = if let Some(alliance_id) = opt_alliance_id {
                let opt_alliance = alliances.iter().find(|a| a.id == alliance_id);
                if let Some(alliance) = opt_alliance {
                    Some((alliance_id, Rc::clone(alliance)))
                } else {
                    None
                }
            } else {
                None
            };

            re.push(Rc::new(Player {
                id,
                name,
                alliance: alliance_tuple,
                points,
                rank,
                towns,
            }))
        }
        return Ok(re);
    }

    fn parse_towns(
        data: String,
        players: &[Rc<Player>],
        islands: &[Rc<Island>],
        offsets: &[Rc<Offset>],
    ) -> anyhow::Result<Vec<Rc<BackendTown>>> {
        let lines: Vec<&str> = data.lines().collect();
        let mut re = Vec::with_capacity(lines.len());
        for line in lines {
            let mut values = line.split(',');

            /*
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

            */

            let id = values
                .next()
                .with_context(|| format!("No town id in {line}"))?
                .parse()
                .with_context(|| format!("No town id in {line} that can be parsed as int"))?;
            let opt_player_id: Option<u32> = {
                let text = values
                    .next()
                    .with_context(|| format!("No town player id in {line}"))?;
                if text.is_empty() {
                    None
                } else {
                    Some(text.parse().with_context(|| {
                        format!("No town player id in {line} that can be parsed as int")
                    })?)
                }
            };
            let name = {
                let text = values
                    .next()
                    .with_context(|| format!("No town name in {line}"))?;
                let decoded = form_urlencoded::parse(text.as_bytes())
                    .map(|(key, val)| [key, val].concat())
                    .collect::<String>();
                decoded
            };
            let x = values
                .next()
                .with_context(|| format!("No town x in {line}"))?
                .parse()
                .with_context(|| format!("No town x in {line} that can be parsed as int"))?;
            let y = values
                .next()
                .with_context(|| format!("No town y in {line}"))?
                .parse()
                .with_context(|| format!("No town y in {line} that can be parsed as int"))?;
            let slot_number = values
                .next()
                .with_context(|| format!("No town slot_number in {line}"))?
                .parse()
                .with_context(|| {
                    format!("No town slot_number in {line} that can be parsed as int")
                })?;
            let points = values
                .next()
                .with_context(|| format!("No town points in {line}"))?
                .parse()
                .with_context(|| format!("No town points in {line} that can be parsed as int"))?;

            // let alliance_tuple = if let Some(alliance_id) = opt_alliance_id {
            //     let opt_alliance = alliances.iter().find(|a| a.id == alliance_id);
            //     if let Some(alliance) = opt_alliance {
            //         Some((alliance_id, Rc::clone(alliance)))
            //     } else {
            //         None
            //     }
            // } else {
            //     None
            // };

            // get actual player from the player id
            let player_tuple = if let Some(player_id) = opt_player_id {
                let opt_player = players.iter().find(|p| p.id == player_id);
                if let Some(player) = opt_player {
                    Some((player_id, Rc::clone(player)))
                } else {
                    None
                }
            } else {
                None
            };

            // get actual island from x and y
            let island_tuple = (x, y, {
                let opt_island = islands.iter().find(|i| i.x == x && i.y == y);
                if let Some(island) = opt_island {
                    Rc::clone(island)
                } else {
                    Rc::clone(&islands[0])
                }
            });

            // get the offset from the offset list from slot_number
            let offset_tuple = (slot_number, {
                let opt_offset = offsets.iter().find(|o| o.slot_number == slot_number);
                if let Some(offset) = opt_offset {
                    Rc::clone(offset)
                } else {
                    Rc::clone(&offsets[0])
                }
            });

            // compute actual x
            let actual_x = x as f32 + offset_tuple.1.x as f32 / 125f32;
            let actual_y = y as f32 + offset_tuple.1.y as f32 / 125f32;

            re.push(Rc::new(BackendTown {
                id,
                name,
                points,
                player: player_tuple,
                island: island_tuple,
                offset: offset_tuple,
                actual_x,
                actual_y,
            }))
        }
        return Ok(re);
    }
}

/*








*/

impl Database {
    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let conn = rusqlite::Connection::open(path)
            .with_context(|| format!("Failed to open database with filename {path:?}"))?;
        Ok(Self { connection: conn })
    }

    pub fn create_for_world(
        server_id: &str,
        filename: Option<&Path>,
        sender: &mpsc::Sender<MessageToView>,
        ctx: &egui::Context,
    ) -> anyhow::Result<Self> {
        let reqwest_client = make_client();

        let mut conn = if let Some(path) = filename {
            rusqlite::Connection::open(path)
                .with_context(|| format!("Failed to open database with filename {path:?}"))?
        } else {
            rusqlite::Connection::open_in_memory().context("Failed to open in memory database")?
        };
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

        // TODO for a proper optimization we should build a new table that pre-performs the list of joins we do in every single query.
        //      only if we want to support more complex queries, which are not based on towns (but instead users or alliances) do we
        //      even need the DB to be split up into different tables.
        // The following code did in fact lead to worse performance....
        // Severely worse performance...
        // optimize the DB
        // make the DB automatically optimize. Can't set analysis_limit though, because it crashed the DB
        // let _result = conn.execute("PRAGMA optimize", [])?;
        // let _result = conn.query_row("PRAGMA optimize(-1)", [], |row| {
        //     println!("{row:?}");
        //     Ok(())
        // });

        // // create indices
        // let _result = conn.execute("CREATE INDEX towns_slot ON towns (slot_number ASC);", [])?;
        // let _result = conn.execute("CREATE INDEX towns_x ON towns (island_x ASC);", [])?;
        // let _result = conn.execute("CREATE INDEX towns_y ON towns (island_y ASC);", [])?;
        // let _result = conn.execute("CREATE INDEX islands_x ON islands (x ASC);", [])?;
        // let _result = conn.execute("CREATE INDEX islands_y ON islands (y ASC);", [])?;

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
