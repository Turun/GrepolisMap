use crate::message::Town;

/// contains all the data required to draw the ui.
pub struct Data {
    pub server_id: String,
    pub canvas: CanvasData,
    pub towns_all: Vec<Town>,
    pub towns_shown: Vec<Town>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            server_id: String::from("de99"),
            canvas: CanvasData {
                center: egui::Pos2 { x: 500.0, y: 500.0 },
                zoom: egui::Vec2 {
                    x: 1000.0,
                    y: 1000.0,
                },
            },
            towns_all: Vec::new(),
            towns_shown: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct CanvasData {
    pub center: egui::Pos2,
    pub zoom: egui::Vec2,
}
