#[derive(Debug, Clone)]
pub struct Town {
    pub id: u32,
    pub player_id: Option<u32>,
    pub player_name: Option<String>,
    pub alliance_name: Option<String>,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub slot_number: u8,
    pub points: u16,
}
