use std::ops::{Add, Div, Mul, Sub};

use crate::message::Town;

/// contains all the data required to draw the ui.
pub struct Data {
    pub server_id: String,
    pub canvas: Option<CanvasData>,
    pub drop_down_string: String,
    pub towns_all: Vec<Town>,
    pub towns_shown: Vec<Town>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            server_id: String::from("de99"),
            canvas: None,
            drop_down_string: String::new(),
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

    pub fn world_to_screen<T>(&self, world: T) -> T
    where
        T: Mul<f32, Output = T>,
        T: Sub<egui::Vec2, Output = T>,
    {
        return self.scale_world_to_screen(world - self.world_offset_px);
    }

    pub fn screen_to_world<T>(&self, screen: T) -> T
    where
        T: Div<f32, Output = T>,
        T: Add<egui::Vec2, Output = T>,
    {
        return self.scale_screen_to_world(screen) + self.world_offset_px;
    }

    pub fn scale_screen_to_world<T>(&self, screen: T) -> T
    where
        T: Div<f32, Output = T>,
    {
        return screen / self.zoom;
    }

    pub fn scale_world_to_screen<T>(&self, world: T) -> T
    where
        T: Mul<f32, Output = T>,
    {
        return world * self.zoom;
    }
}
