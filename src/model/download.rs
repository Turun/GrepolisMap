use super::database::{Alliance, BackendTown, DataTable, Island, Offset, Player};
use super::offset_data;
use crate::message::{MessageToView, Progress};
use anyhow::Context;
use reqwest;
use std::collections::HashMap;
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

        let towns = towns.into_values().collect();
        Ok(Self { towns })
    }

    fn make_offsets() -> HashMap<(u8, u8), Rc<Offset>> {
        let lines: Vec<&str> = offset_data::OFFSET_DATA.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
        for line in lines {
            let mut values = line.split(",");
            let typ: u8 = values.next().unwrap().parse().unwrap();
            let x: u16 = values.next().unwrap().parse().unwrap();
            let y: u16 = values.next().unwrap().parse().unwrap();
            let slot_number: u8 = values.next().unwrap().parse().unwrap();
            let _duplicate = re.insert(
                (typ, slot_number),
                Rc::new(Offset {
                    typ,
                    x,
                    y,
                    slot_number,
                }),
            );
        }
        return re;
    }

    fn parse_alliances(data: String) -> anyhow::Result<HashMap<u32, Rc<Alliance>>> {
        let lines: Vec<&str> = data.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
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
            let _duplicate = re.insert(
                id,
                Rc::new(Alliance {
                    id,
                    name,
                    points,
                    towns,
                    members,
                    rank,
                }),
            );
        }
        return Ok(re);
    }

    fn parse_islands(data: String) -> anyhow::Result<HashMap<(u16, u16), Rc<Island>>> {
        let lines: Vec<&str> = data.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
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
            let _duplicate = re.insert(
                (x, y),
                Rc::new(Island {
                    id,
                    x,
                    y,
                    typ,
                    towns,
                    ressource_plus,
                    ressource_minus,
                }),
            );
        }
        return Ok(re);
    }

    fn parse_players(
        data: String,
        alliances: &HashMap<u32, Rc<Alliance>>,
    ) -> anyhow::Result<HashMap<u32, Rc<Player>>> {
        let lines: Vec<&str> = data.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
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
                let opt_alliance = alliances.get(&alliance_id);
                if let Some(alliance) = opt_alliance {
                    Some((alliance_id, Rc::clone(alliance)))
                } else {
                    None
                }
            } else {
                None
            };

            let _duplicate = re.insert(
                id,
                Rc::new(Player {
                    id,
                    name,
                    alliance: alliance_tuple,
                    points,
                    rank,
                    towns,
                }),
            );
        }
        return Ok(re);
    }

    fn parse_towns(
        data: String,
        players: &HashMap<u32, Rc<Player>>,
        islands: &HashMap<(u16, u16), Rc<Island>>,
        offsets: &HashMap<(u8, u8), Rc<Offset>>,
    ) -> anyhow::Result<HashMap<u32, Rc<BackendTown>>> {
        let lines: Vec<&str> = data.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
        for line in lines {
            let mut values = line.split(',');

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

            // get actual player from the player id
            let player_tuple = if let Some(player_id) = opt_player_id {
                let opt_player = players.get(&player_id);
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
                let opt_island = islands.get(&(x, y));
                if let Some(island) = opt_island {
                    Rc::clone(island)
                } else {
                    // every town _needs_ a corresponding island. So if there is no matching one found we take the first we get.
                    let (_key, value) = islands.iter().next().unwrap();
                    Rc::clone(value)
                }
            });

            // get the offset from the offset list from slot_number
            let offset_tuple = (slot_number, {
                let opt_offset = offsets.get(&(island_tuple.2.typ, slot_number));
                if let Some(offset) = opt_offset {
                    Rc::clone(offset)
                } else {
                    // correspondingly, every town also _needs_ an offset tuple.
                    let (_key, value) = offsets.iter().next().unwrap();
                    Rc::clone(value)
                }
            });

            // compute actual x
            let actual_x = x as f32 + offset_tuple.1.x as f32 / 125f32;
            let actual_y = y as f32 + offset_tuple.1.y as f32 / 125f32;

            let _duplicate = re.insert(
                id,
                Rc::new(BackendTown {
                    id,
                    name,
                    points,
                    player: player_tuple,
                    island: island_tuple,
                    offset: offset_tuple,
                    actual_x,
                    actual_y,
                }),
            );
        }
        return Ok(re);
    }
}
