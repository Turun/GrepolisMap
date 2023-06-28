use std::{
    collections::HashMap,
    ops::{Add, Div, Mul, Sub},
};

use crate::towns::{ConstraintType, Town, TownSelection};

#[derive(Clone)]
pub struct DefaultTownGroup {
    pub enabled: bool,
    pub color: egui::Color32,
}

/// contains all the data required to draw the ui.
pub struct Data {
    pub server_id: String,
    pub canvas: Option<CanvasData>,

    pub settings_all: DefaultTownGroup,
    pub settings_ghosts: DefaultTownGroup,

    pub selections: Vec<TownSelection>,

    pub all_towns: Vec<Town>,
    pub ghost_towns: Vec<Town>,

    pub dropdown_values: HashMap<ConstraintType, Vec<String>>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            server_id: String::from("de99"),
            canvas: None,
            all_towns: Vec::new(),
            ghost_towns: Vec::new(),
            selections: vec![TownSelection::default()],
            settings_ghosts: DefaultTownGroup {
                enabled: true,
                color: egui::Color32::RED,
            },
            settings_all: DefaultTownGroup {
                enabled: true,
                color: egui::Color32::from_rgb(48, 48, 48),
            },
            dropdown_values: HashMap::new(),
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

pub struct ViewPortFilter {
    world_l: f32,
    world_r: f32,
    world_b: f32,
    world_t: f32,
}

impl ViewPortFilter {
    pub fn new(canvas: &CanvasData, screen_rect: egui::Rect) -> Self {
        let top_left = canvas.screen_to_world(screen_rect.left_top().to_vec2());
        let bot_right = canvas.screen_to_world(screen_rect.right_bottom().to_vec2());
        Self {
            world_l: top_left.x,
            world_r: bot_right.x,
            world_t: top_left.y,
            world_b: bot_right.y,
        }
    }

    pub fn town_in_viewport(&self, town: &Town) -> bool {
        self.world_l < town.x
            && town.x < self.world_r
            && self.world_t < town.y
            && town.y < self.world_b
    }

    pub fn x_in_viewport(&self, x: f32) -> bool {
        self.world_l < x && x < self.world_r
    }

    pub fn y_in_viewport(&self, y: f32) -> bool {
        self.world_t < y && y < self.world_b
    }
}
