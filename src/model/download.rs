use super::database::{Alliance, BackendTown, DataTable, Island, Offset, Player};
use super::{offset_data, APIResponse};
use anyhow::Context;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/*
Forum threads:
https://en.forum.grepolis.com/index.php?threads/world-data-api.52/
https://en.forum.grepolis.com/index.php?threads/changes-to-world-data.5589/

Grepolis - World Data

We have written this small guide for other developers who wish to utilise the Grepolis World Data to create their own websites. A good example of what this data can be used to do is Grepo Stats. All data is currently available in the JSON format. Currently world data is updated hourly. Each world's data will be updated at a random starting minute.

Note: Please use the compressed world data whenever possible.
Player data
- players.json
- players.json.gz

The player data contains the following information:

* id - The internal ID of the player
* name - The player's name
* alliance_id - The player's internal alliance ID (see alliances.json)
* points - The player's current amount of points
* rank - The player's current rank
* towns - The player's current amount of cities

Alliance data
- alliances.json
- alliances.json.gz

The alliance data contains the following information:

* id - The internal ID of the alliance
* name - The alliances's name
* points - The alliance's current amount of points
* rank - The alliance's current rank
* towns - The alliances's current amount of cities
* members - The alliances's current member count

Town data
- towns.json
- towns.json.gz

The town data contains the following information:

* id - The internal ID of the town
* player_id - The owner of the town. NULL if no owner
* name - The town's name
* island_x - The X coordinate of the island the town is on
* island_y - The Y coordinate of the island the town is on
* number_on_island - The position of the town on it's island
* points - The town's points

Island data *large file*
- islands.json
- islands.json.gz

The islands data file contains the information for the world's islands. Please note that this is a large file and that once a world has started, it's contents will not change. Therefore, you do not need to keep downloading this file!
The island data contains the following information:

* id - The internal ID of the island
* x - The X position of the island
* y - The Y position of the island
* island_id - The internal number of the island, or, what 'type/size/shape' of island this is. 1-10 = player islands with farm towns, 11-16 = uninhabited islands, 17-21 = rocks

Player kill data
- player_kills.json
- player_kills.json.gz

The player kill data contains information about the player 'kill' ranking. There is a record for each player who is ranked. Each record is an array of objects, with the keys 'all', 'att', 'def'. These keys mean: 'Overall', 'As attacker', 'As defender'.
Each of the aforementioned objects contain the following data:

* rank - The rank for the specific type
* player_id - The internal ID of the player
* points - The points/score

Alliance kill data
- alliance_kills.json
- alliance_kills.json.gz

The alliance kill data contains information about the alliance 'kill' ranking. There is a record for each alliance that is ranked. Each record is an array of objects, with the keys 'all', 'att', 'def'. These keys mean: 'Overall', 'As attacker', 'As defender'.
Each of the aforementioned objects contain the following data:

* rank - The rank for the specific type
* alliance_id - The internal ID of the alliance
* points - The points/score

Colonisation data
- conquers.json
- conquers.json.gz

The colonisation data contains the following information:

* town_id - The town's internal ID
* time - The UNIX timestamp of the conquer
* new_player_id - The player who colonised the town
* old_player_id - If involved, the player who lost the town. Otherwise, NULL
* new_ally_id - If in an alliance, the alliance ID of the player who colonised the town. Otherwise, NULL
* old_ally_id - If in an alliance, the alliance ID of the player who lost the town. Otherwise, NULL
* town_points - The town's points at the time of the colonisation

Building, unit and research information
- units.json
- units.json.gz
- buildings.json
- buildings.json.gz
- researches.json
- researches.json.gz

*/

impl DataTable {
    pub fn get_api_results(api_results: &Arc<Mutex<APIResponse>>) {
        let server_id = api_results.lock().unwrap().for_server.clone();

        #[cfg(target_arch = "wasm32")]
        let base_url = format!("https://reflector.gmap.turun.de/{server_id}/");
        #[cfg(not(target_arch = "wasm32"))]
        let base_url = format!("https://{server_id}.grepolis.com/data/");

        let req_players = ehttp::Request::get(base_url.clone() + "players.txt");
        let these_api_results = Arc::clone(api_results);
        ehttp::fetch(req_players, move |response| {
            let text = String::from_utf8(response.unwrap().bytes).unwrap();
            these_api_results.lock().unwrap().players = Some(text);
        });

        let req_players = ehttp::Request::get(base_url.clone() + "alliances.txt");
        let these_api_results = Arc::clone(api_results);
        ehttp::fetch(req_players, move |response| {
            let text = String::from_utf8(response.unwrap().bytes).unwrap();
            these_api_results.lock().unwrap().alliances = Some(text);
        });

        let req_players = ehttp::Request::get(base_url.clone() + "towns.txt");
        let these_api_results = Arc::clone(api_results);
        ehttp::fetch(req_players, move |response| {
            let text = String::from_utf8(response.unwrap().bytes).unwrap();
            these_api_results.lock().unwrap().towns = Some(text);
        });

        let req_players = ehttp::Request::get(base_url.clone() + "islands.txt");
        let these_api_results = Arc::clone(api_results);
        ehttp::fetch(req_players, move |response| {
            let text = String::from_utf8(response.unwrap().bytes).unwrap();
            these_api_results.lock().unwrap().islands = Some(text);
        });
    }

    pub fn create_for_world(api_response: APIResponse) -> Self {
        // TODO: we need to massively improve the way we handle errors here. Crashing the entire backend if one line in
        // one input file is unexpected is not a good solution. We need more fine grained error handling.
        let offsets = Self::make_offsets();
        let (bl_alliances, alliances) = Self::parse_alliances(&api_response.alliances.unwrap());
        let (bl_islands, islands) = Self::parse_islands(&api_response.islands.unwrap());
        let (bl_players, players) = Self::parse_players(&api_response.players.unwrap(), &alliances);
        let (bl_towns, towns) =
            Self::parse_towns(&api_response.towns.unwrap(), &players, &islands, &offsets);
        let towns = towns.into_values().collect();

        // TODO: do something with the bad lines information
        let total_bad_lines = bl_alliances + bl_islands + bl_players + bl_towns;
        if total_bad_lines > 0 {
            eprintln!("Got {total_bad_lines} bad lines in api response.");
        }
        Self { towns }
    }

    fn make_offsets() -> HashMap<(u8, u8), Rc<Offset>> {
        let lines: Vec<&str> = offset_data::OFFSET_DATA.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
        for line in lines {
            let mut values = line.split(',');
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

    fn parse_alliances(data: &str) -> (u32, HashMap<u32, Rc<Alliance>>) {
        fn parse_line(line: &str) -> anyhow::Result<(u32, Alliance)> {
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
            return Ok((
                id,
                Alliance {
                    id,
                    name,
                    points,
                    towns,
                    members,
                    rank,
                },
            ));
        }

        let mut bad_lines = 0;
        let lines: Vec<&str> = data.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
        for line in lines {
            if let Ok((id, alliance)) = parse_line(line) {
                let _duplicate = re.insert(id, Rc::new(alliance));
            } else {
                bad_lines += 1;
            }
        }
        return (bad_lines, re);
    }

    fn parse_islands(data: &str) -> (u32, HashMap<(u16, u16), Rc<Island>>) {
        fn parse_line(line: &str) -> anyhow::Result<(u16, u16, Island)> {
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
            return Ok((
                x,
                y,
                Island {
                    id,
                    x,
                    y,
                    typ,
                    towns,
                    ressource_plus,
                    ressource_minus,
                },
            ));
        }

        let mut bad_lines = 0;
        let lines: Vec<&str> = data.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
        for line in lines {
            if let Ok((x, y, island)) = parse_line(line) {
                let _duplicate = re.insert((x, y), Rc::new(island));
            } else {
                bad_lines += 1;
            }
        }
        return (bad_lines, re);
    }

    fn parse_players(
        data: &str,
        alliances: &HashMap<u32, Rc<Alliance>>,
    ) -> (u32, HashMap<u32, Rc<Player>>) {
        fn parse_line(
            line: &str,
            alliances: &HashMap<u32, Rc<Alliance>>,
        ) -> anyhow::Result<(u32, Player)> {
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
                opt_alliance.map(|alliance| (alliance_id, Rc::clone(alliance)))
            } else {
                None
            };

            return Ok((
                id,
                Player {
                    id,
                    name,
                    alliance: alliance_tuple,
                    points,
                    rank,
                    towns,
                },
            ));
        }

        let mut bad_lines = 0;
        let lines: Vec<&str> = data.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
        for line in lines {
            if let Ok((id, player)) = parse_line(line, alliances) {
                let _duplicate = re.insert(id, Rc::new(player));
            } else {
                bad_lines += 1;
            }
        }
        return (bad_lines, re);
    }

    fn parse_towns(
        data: &str,
        players: &HashMap<u32, Rc<Player>>,
        islands: &HashMap<(u16, u16), Rc<Island>>,
        offsets: &HashMap<(u8, u8), Rc<Offset>>,
    ) -> (u32, HashMap<u32, Rc<BackendTown>>) {
        fn parse_line(
            line: &str,
            players: &HashMap<u32, Rc<Player>>,
            islands: &HashMap<(u16, u16), Rc<Island>>,
            offsets: &HashMap<(u8, u8), Rc<Offset>>,
        ) -> anyhow::Result<(u32, BackendTown)> {
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
                opt_player.map(|player| (player_id, Rc::clone(player)))
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
            let actual_x = f32::from(x) + f32::from(offset_tuple.1.x) / 125f32;
            let actual_y = f32::from(y) + f32::from(offset_tuple.1.y) / 125f32;

            return Ok((
                id,
                BackendTown {
                    id,
                    name,
                    points,
                    player: player_tuple,
                    island: island_tuple,
                    offset: offset_tuple,
                    actual_x,
                    actual_y,
                },
            ));
        }

        let mut bad_lines = 0;
        let lines: Vec<&str> = data.lines().collect();
        let mut re = HashMap::with_capacity(lines.len());
        for line in lines {
            if let Ok((id, town)) = parse_line(line, players, islands, offsets) {
                let _duplicate = re.insert(id, Rc::new(town));
            } else {
                // TODO: dont use a counter, use a list that contains the lines themselves
                bad_lines += 1;
            }
        }
        return (bad_lines, re);
    }
}
