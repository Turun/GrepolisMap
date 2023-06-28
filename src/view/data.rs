use crate::message::Town;

/// contains all the data required to draw the ui.
pub struct Data {
    pub server_id: String,
    pub canvas: Option<CanvasData>,
    pub towns_all: Vec<Town>,
    pub towns_shown: Vec<Town>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            server_id: String::from("de99"),
            canvas: None,
            towns_all: Vec::new(),
            towns_shown: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct CanvasData {
    /// distance from top left of canvas to top left of grepolis coordinates
    pub world_offset_px: egui::Vec2,
    pub zoom: f32,
}

impl CanvasData {
    pub fn new(top_left: egui::Vec2) -> Self {
        Self {
            world_offset_px: top_left,
            zoom: 1.0,
        }
    }

    pub fn world_to_screen(&self, world: egui::Vec2) -> egui::Vec2 {
        return self.scale_world_to_screen(world - self.world_offset_px);
    }

    pub fn screen_to_world(&self, screen: egui::Vec2) -> egui::Vec2 {
        return self.scale_screen_to_world(screen) + self.world_offset_px;
    }

    pub fn scale_screen_to_world(&self, screen: egui::Vec2) -> egui::Vec2 {
        return screen / self.zoom;
    }

    pub fn scale_world_to_screen(&self, world: egui::Vec2) -> egui::Vec2 {
        return world * self.zoom;
    }
}
