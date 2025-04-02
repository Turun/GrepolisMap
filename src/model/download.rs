use super::database::{Alliance, BackendTown, DataTable, Island, Offset, Player};
use super::{offset_data, APIResponse};
use crate::message::{MessageToView, Progress, Server};
use anyhow::{anyhow, Context};
use reqwest;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};

impl DataTable {
    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        todo!();
    }

    pub fn get_api_results(api_results: Arc<Mutex<APIResponse>>) {
        let server_id = api_results.lock().unwrap().for_server.clone();

        #[cfg(target_arch = "wasm32")]
        let base_url = format!("https://reflector.gmap.turun.de/{server_id}/");
        #[cfg(not(target_arch = "wasm32"))]
        let base_url = format!("https://{server_id}.grepolis.com/data/");

        let req_players = ehttp::Request::get(base_url.clone() + "players.txt");
        let these_api_results = Arc::clone(&api_results);
        ehttp::fetch(req_players, move |response| {
            let text = String::from_utf8(response.unwrap().bytes).unwrap();
            these_api_results.lock().unwrap().players = Some(text);
        });

        let req_players = ehttp::Request::get(base_url.clone() + "alliances.txt");
        let these_api_results = Arc::clone(&api_results);
        ehttp::fetch(req_players, move |response| {
            let text = String::from_utf8(response.unwrap().bytes).unwrap();
            these_api_results.lock().unwrap().alliances = Some(text);
        });

        let req_players = ehttp::Request::get(base_url.clone() + "towns.txt");
        let these_api_results = Arc::clone(&api_results);
        ehttp::fetch(req_players, move |response| {
            let text = String::from_utf8(response.unwrap().bytes).unwrap();
            these_api_results.lock().unwrap().towns = Some(text);
        });

        let req_players = ehttp::Request::get(base_url.clone() + "islands.txt");
        let these_api_results = Arc::clone(&api_results);
        ehttp::fetch(req_players, move |response| {
            let text = String::from_utf8(response.unwrap().bytes).unwrap();
            these_api_results.lock().unwrap().islands = Some(text);
        });
    }

    pub fn create_for_world(
        api_response: APIResponse,
        filename: Option<&Path>,
    ) -> anyhow::Result<Self> {
        // TODO: we need to massively improve the way we handle errors here. Crashing the entire backend if one line in
        // one input file is unexpected is not a good solution. We need more fine grained error handling.
        if let Some(path) = filename {
            // TODO: load from file and return immediately
        };

        // sender
        //     .send(MessageToView::Loading(Progress::Started))
        //     .context("Failed to send progressupdate 1 to view")?;
        // ctx.request_repaint();

        let offsets = Self::make_offsets();
        // sender
        //     .send(MessageToView::Loading(Progress::IslandOffsets))
        //     .context("Failed to send progressupdate 2 to view")?;
        // ctx.request_repaint();

        let alliances = Self::parse_alliances(api_response.alliances.unwrap())?;
        // sender
        //     .send(MessageToView::Loading(Progress::Alliances))
        //     .context("Failed to send progressupdate 3 to view")?;
        // ctx.request_repaint();

        let islands = Self::parse_islands(api_response.islands.unwrap())?;
        // sender
        //     .send(MessageToView::Loading(Progress::Islands))
        //     .context("Failed to send progressupdate 4 to view")?;
        // ctx.request_repaint();

        let players = Self::parse_players(api_response.players.unwrap(), &alliances)?;
        // sender
        //     .send(MessageToView::Loading(Progress::Players))
        //     .context("Failed to send progressupdate 5 to view")?;
        // ctx.request_repaint();

        let towns = Self::parse_towns(api_response.towns.unwrap(), &players, &islands, &offsets)?;
        // sender
        //     .send(MessageToView::Loading(Progress::Towns))
        //     .context("Failed to send progressupdate 6 to view")?;
        // ctx.request_repaint();

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
