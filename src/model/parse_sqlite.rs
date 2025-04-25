//! The new rust backend can't read the old sqlite files. The following code takes a saved sqlite
// file, reads it and transforms the content into a valid API Response struct so the new backend can
// work with old save files.

use anyhow::Context;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::Connection;

use crate::storage::SavedDB;

use super::APIResponse;

#[allow(clippy::too_many_lines)]
pub fn sqlite_to_apiresponse(db: SavedDB) -> anyhow::Result<APIResponse> {
    let path = db.path.clone();
    let conn =
        Connection::open(&path).with_context(|| format!("Failed to open database at {path:?}"))?;

    // 1) players.txt
    let mut stmt = conn
        .prepare(
            "SELECT player_id, name, alliance_id, points, rank, towns
         FROM players
         ORDER BY player_id",
        )
        .context("Failed to prepare players SELECT")?;
    let players = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let alliance_id: Option<i64> = row.get(2)?;
            let pts: i64 = row.get(3)?;
            let rank: i64 = row.get(4)?;
            let towns: i64 = row.get(5)?;

            // URL-encode exactly as form_urlencoded::parse would decode
            let name_enc = utf8_percent_encode(&name, NON_ALPHANUMERIC).to_string();
            let alliance = alliance_id.map(|a| a.to_string()).unwrap_or_default();

            Ok(format!("{id},{name_enc},{alliance},{pts},{rank},{towns}"))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let players_txt = players.join("\n");

    // 2) alliances.txt
    let mut stmt = conn
        .prepare(
            "SELECT alliance_id, name, points, towns, members, rank
         FROM alliances
         ORDER BY alliance_id",
        )
        .context("Failed to prepare alliances SELECT")?;
    let alliances = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let name: String = row.get(1)?;
            let pts: i64 = row.get(2)?;
            let towns: i64 = row.get(3)?;
            let memb: i64 = row.get(4)?;
            let rank: i64 = row.get(5)?;

            let name_enc = utf8_percent_encode(&name, NON_ALPHANUMERIC).to_string();

            Ok(format!("{id},{name_enc},{pts},{towns},{memb},{rank}"))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let alliances_txt = alliances.join("\n");

    // 3) towns.txt
    let mut stmt = conn
        .prepare(
            "SELECT town_id, player_id, name, island_x, island_y, slot_number, points
         FROM towns
         ORDER BY town_id",
        )
        .context("Failed to prepare towns SELECT")?;
    let towns = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let player_id: Option<i64> = row.get(1)?;
            let name: String = row.get(2)?;
            let ix: i64 = row.get(3)?;
            let iy: i64 = row.get(4)?;
            let slot: i64 = row.get(5)?;
            let pts: i64 = row.get(6)?;

            let name_enc = utf8_percent_encode(&name, NON_ALPHANUMERIC).to_string();
            let player = player_id.map(|p| p.to_string()).unwrap_or_default();

            Ok(format!("{id},{player},{name_enc},{ix},{iy},{slot},{pts}"))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let towns_txt = towns.join("\n");

    // 4) islands.txt
    let mut stmt = conn
        .prepare(
            "SELECT island_id, x, y, type, towns, ressource_plus, ressource_minus
         FROM islands
         ORDER BY island_id",
        )
        .context("Failed to prepare islands SELECT")?;
    let islands = stmt
        .query_map([], |row| {
            let id: i64 = row.get(0)?;
            let x: i64 = row.get(1)?;
            let y: i64 = row.get(2)?;
            let typ: i64 = row.get(3)?;
            let towns: i64 = row.get(4)?;
            let plus: String = row.get(5)?;
            let minus: String = row.get(6)?;

            Ok(format!("{id},{x},{y},{typ},{towns},{plus},{minus}"))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let islands_txt = islands.join("\n");

    Ok(APIResponse {
        for_server: db.server_str,
        filename: Some(db.path.clone()),
        timestamp: db.date,
        players: Some(players_txt),
        alliances: Some(alliances_txt),
        towns: Some(towns_txt),
        islands: Some(islands_txt),
    })
}
