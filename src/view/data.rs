use std::{
    collections::BTreeMap,
    ops::{Add, Div, Mul, Sub},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

use crate::selection::TownSelection;
#[cfg(not(target_arch = "wasm32"))]
use crate::storage::SavedDB;
use crate::town::Town;
use crate::view::preferences::{DarkModePref, Preferences};

pub const ALL_TOWNS_DARK: egui::Color32 = egui::Color32::from_gray(60);
pub const ALL_TOWNS_LIGHT: egui::Color32 = egui::Color32::from_gray(180);

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DefaultTownGroup {
    pub enabled: bool,
    pub color: egui::Color32,
}

/// contains all the data required to draw the ui.
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Data {
    pub server_id: String,

    #[serde(skip)]
    pub canvas: Option<CanvasData>,

    pub settings_all: DefaultTownGroup,
    pub settings_ghosts: DefaultTownGroup,

    pub selections: Vec<TownSelection>,

    #[serde(skip)]
    pub all_towns: Arc<Vec<Town>>,
    #[serde(skip)]
    pub ghost_towns: Arc<Vec<Town>>,

    #[serde(skip)]
    #[cfg(not(target_arch = "wasm32"))]
    pub saved_db: BTreeMap<String, Vec<SavedDB>>,

    #[serde(skip)]
    #[cfg(target_arch = "wasm32")]
    pub url: Option<String>,

    pub preferences: Preferences,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            server_id: String::from("de99"),
            canvas: None,
            all_towns: Arc::new(Vec::new()),
            ghost_towns: Arc::new(Vec::new()),
            selections: vec![TownSelection::default()],
            settings_ghosts: DefaultTownGroup {
                enabled: true,
                color: egui::Color32::RED,
            },
            settings_all: DefaultTownGroup {
                enabled: true,
                color: ALL_TOWNS_DARK,
            },
            #[cfg(not(target_arch = "wasm32"))]
            saved_db: BTreeMap::new(),
            #[cfg(target_arch = "wasm32")]
            url: None,
            preferences: Preferences::default(),
        }
    }
}

impl Data {
    pub fn apply_darkmode(&mut self, ctx: &egui::Context, mode: DarkModePref) {
        // update preferences
        self.preferences.darkmode = mode;

        // and apply it,
        // but only change the town color if the user didn't set a non-default color
        match mode {
            DarkModePref::FollowSystem => {
                /* do nothing. This is integrated into eframe and uses winit to get the
                system theme, we can't clone that logic here without involving winit
                ourselves. Considering we basically save _everything_ across app restarts
                I don't think it's too big of an ask to have the user restart the app.  */
            }
            DarkModePref::Dark => {
                ctx.set_visuals(egui::Visuals::dark());
                if self.settings_all.color == ALL_TOWNS_LIGHT {
                    self.settings_all.color = ALL_TOWNS_DARK;
                }
            }
            DarkModePref::Light => {
                ctx.set_visuals(egui::Visuals::light());
                if self.settings_all.color == ALL_TOWNS_DARK {
                    self.settings_all.color = ALL_TOWNS_LIGHT;
                }
            }
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Debug, Clone)]
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
        self.scale_world_to_screen(world - self.world_offset_px)
    }

    pub fn screen_to_world<T>(&self, screen: T) -> T
    where
        T: Div<f32, Output = T>,
        T: Add<egui::Vec2, Output = T>,
    {
        self.scale_screen_to_world(screen) + self.world_offset_px
    }

    pub fn scale_screen_to_world<T>(&self, screen: T) -> T
    where
        T: Div<f32, Output = T>,
    {
        screen / self.zoom
    }

    pub fn scale_world_to_screen<T>(&self, world: T) -> T
    where
        T: Mul<f32, Output = T>,
    {
        world * self.zoom
    }
}

#[allow(clippy::struct_field_names)]
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
