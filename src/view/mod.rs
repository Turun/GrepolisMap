mod data;
pub(crate) mod state;

use std::sync::mpsc;

use egui::{ProgressBar, Shape};

use crate::message::{
    MessageToModel, MessageToView, Progress, Server, Town, TownConstraint, TownSelection,
};
use crate::view::data::{CanvasData, Data};
use crate::view::state::State;

pub struct View {
    ui_state: State,
    ui_data: Data,
    channel_presenter_rx: mpsc::Receiver<MessageToView>,
    channel_presenter_tx: mpsc::Sender<MessageToModel>,
}

impl View {
    pub fn new(rx: mpsc::Receiver<MessageToView>, tx: mpsc::Sender<MessageToModel>) -> Self {
        Self {
            ui_state: State::Uninitialized(Progress::None),
            ui_data: Data::default(),
            channel_presenter_rx: rx,
            channel_presenter_tx: tx,
        }
    }

    pub fn start(self) {
        let native_options = eframe::NativeOptions::default();
        let result = eframe::run_native(
            "Grepolis Map",
            native_options,
            Box::new(|cc| Box::new(self)),
        );
    }

    fn ui_uninitialized(&mut self, ctx: &egui::Context, progress: Progress) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Server ID");
                    ui.text_edit_singleline(&mut self.ui_data.server_id);
                });
                if ui
                    .add(egui::Button::new("Load Data for this Server"))
                    .clicked()
                {
                    self.channel_presenter_tx
                        .send(MessageToModel::SetServer(Server {
                            id: self.ui_data.server_id.clone(),
                        }))
                        .expect("Failed to send the SetServer Message to the backend");
                }

                match progress {
                    Progress::None => {}
                    Progress::Started => {
                        ui.add(ProgressBar::new(0.0).text(format!("{:?}", progress)));
                    }
                    Progress::IslandOffsets => {
                        ui.add(ProgressBar::new(0.2).text(format!("{:?}", progress)));
                    }
                    Progress::Alliances => {
                        ui.add(ProgressBar::new(0.4).text(format!("{:?}", progress)));
                    }
                    Progress::Players => {
                        ui.add(ProgressBar::new(0.6).text(format!("{:?}", progress)));
                    }
                    Progress::Towns => {
                        ui.add(ProgressBar::new(0.8).text(format!("{:?}", progress)));
                    }
                    Progress::Islands => {
                        ui.add(ProgressBar::new(1.0).text(format!("{:?}", progress)));
                    }
                }
            });
        });
    }

    fn ui_init(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.heading(String::from("Server:") + &self.ui_data.server_id);
            })
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let (mut response, painter) = ui.allocate_painter(
                    ui.available_size_before_wrap(),
                    egui::Sense::click_and_drag(),
                );

                if let None = self.ui_data.canvas {
                    self.ui_data.canvas =
                        Some(CanvasData::new(-response.rect.left_top().to_vec2()));
                }
                let canvas_data = self.ui_data.canvas.as_mut().unwrap();

                //DRAG
                canvas_data.world_offset_px -=
                    canvas_data.scale_screen_to_world(response.drag_delta());

                // ZOOM
                // as per https://www.youtube.com/watch?v=ZQ8qtAizis4
                let mouse_position_in_world_space_before_zoom_change = {
                    if let Some(mouse_position) = response.hover_pos() {
                        canvas_data.screen_to_world(mouse_position.to_vec2())
                    } else {
                        egui::vec2(0.0, 0.0)
                    }
                };

                let scroll_delta = ctx.input(|input| input.scroll_delta.y);
                if scroll_delta > 0.0 {
                    canvas_data.zoom *= 1.2;
                } else if scroll_delta < 0.0 {
                    canvas_data.zoom /= 1.2;
                }

                let mouse_position_in_world_space_after_zoom_change = {
                    if let Some(mouse_position) = response.hover_pos() {
                        canvas_data.screen_to_world(mouse_position.to_vec2())
                    } else {
                        egui::vec2(0.0, 0.0)
                    }
                };

                canvas_data.world_offset_px += mouse_position_in_world_space_before_zoom_change
                    - mouse_position_in_world_space_after_zoom_change;

                // filter everything that is not visible
                let filter = ViewPortFilter::new(&canvas_data, response.rect);
                let visible_towns_all: Vec<&Town> = self
                    .ui_data
                    .towns_all
                    .iter()
                    .filter(|town| filter.town_in_viewport(town))
                    .collect();
                let visible_towns_shown: Vec<&Town> = self
                    .ui_data
                    .towns_shown
                    .iter()
                    .filter(|town| filter.town_in_viewport(town))
                    .collect();

                // DRAW GRID
                for i in (0..=10).map(|i| i as f32 * 100.0) {
                    // vertical
                    let one = canvas_data.world_to_screen(egui::vec2(0.0, i)).to_pos2();
                    let two = canvas_data.world_to_screen(egui::vec2(1000.0, i)).to_pos2();
                    painter
                        .line_segment([one, two], egui::Stroke::new(2.0, egui::Color32::DARK_GRAY));
                    // horizontal
                    let one = canvas_data.world_to_screen(egui::vec2(i, 0.0)).to_pos2();
                    let two = canvas_data.world_to_screen(egui::vec2(i, 1000.0)).to_pos2();
                    painter
                        .line_segment([one, two], egui::Stroke::new(2.0, egui::Color32::DARK_GRAY));
                }
                if canvas_data.zoom > 5.0 {
                    for i in (0..=100)
                        .map(|i| i as f32 * 10.0)
                        .filter(|&i| filter.x_in_viewport(i) || filter.y_in_viewport(i))
                    {
                        // vertical
                        let one = canvas_data.world_to_screen(egui::vec2(0.0, i)).to_pos2();
                        let two = canvas_data.world_to_screen(egui::vec2(1000.0, i)).to_pos2();
                        painter.add(Shape::dashed_line(
                            &[one, two],
                            egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
                            7.0,
                            7.0,
                        ));
                        // horizontal
                        let one = canvas_data.world_to_screen(egui::vec2(i, 0.0)).to_pos2();
                        let two = canvas_data.world_to_screen(egui::vec2(i, 1000.0)).to_pos2();
                        painter.add(Shape::dashed_line(
                            &[one, two],
                            egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
                            7.0,
                            7.0,
                        ));
                    }
                }

                // DRAW ALL TOWNS
                // towns have a diameter of .25 units, approximately
                for town in &visible_towns_all {
                    painter.circle_filled(
                        canvas_data
                            .world_to_screen(egui::vec2(town.x, town.y))
                            .to_pos2(),
                        1.0 + canvas_data.scale_world_to_screen(0.15),
                        if town.player_id == Some(1495649) {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::from_rgb(25, 200, 100)
                            // egui::Color32::TRANSPARENT
                        },
                    );
                }

                // DRAW GHOST TOWNS
                for town in &visible_towns_shown {
                    painter.circle_filled(
                        canvas_data
                            .world_to_screen(egui::vec2(town.x, town.y))
                            .to_pos2(),
                        2.0 + canvas_data.scale_world_to_screen(0.15),
                        egui::Color32::RED,
                    );
                }

                // POPUP WITH TOWN INFORMATION
                // TODO
                if canvas_data.zoom > 10.0 {
                    let optional_mouse_position = response.hover_pos();
                    response = response.on_hover_ui_at_pointer(|ui| {
                        let position = if let Some(mouse_position) = optional_mouse_position {
                            canvas_data
                                .screen_to_world(mouse_position.to_vec2())
                                .to_pos2()
                        } else {
                            return;
                            // egui::pos2(0.0, 0.0)
                        };
                        ui.label(format!("{:?}", position));

                        if &visible_towns_all.len() < &1usize {
                            return;
                        }
                        let mut closest_town = visible_towns_all[0];
                        let mut closest_distance =
                            position.distance(egui::pos2(closest_town.x, closest_town.y));
                        for town in &visible_towns_all {
                            let distance = position.distance(egui::pos2(town.x, town.y));
                            if distance < closest_distance {
                                closest_distance = distance;
                                closest_town = town;
                            }
                        }

                        ui.label(format!(
                            "{}\nPoints: {}\nPlayer: {}\nDistance: {}",
                            closest_town.name,
                            closest_town.points,
                            "Not yet implemented",
                            closest_distance
                        ));
                    });
                }

                response
            })
        });
    }
}

impl eframe::App for View {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Ok(message) = self.channel_presenter_rx.try_recv() {
            println!("Got Message from Model to View: {}", message);
            match message {
                MessageToView::GotServer(all_towns) => {
                    self.ui_state = State::Show(TownSelection::None);
                    self.ui_data.towns_all = all_towns;
                    self.channel_presenter_tx
                        .send(MessageToModel::FetchTowns(TownSelection::Ghosts))
                        .expect("Failed to send message to model: TownSelection::Ghosts");
                }
                MessageToView::TownList(selection, town_list) => {
                    self.ui_state = State::Show(selection);
                    self.ui_data.towns_shown = town_list;
                }
                MessageToView::Loading(progress) => {
                    self.ui_state = State::Uninitialized(progress);
                }
            }
        }
        let state = self.ui_state.clone();
        match state {
            State::Uninitialized(progress) => self.ui_uninitialized(ctx, progress),
            State::Show(_) => self.ui_init(ctx),
        }
    }
}

struct ViewPortFilter {
    world_l: f32,
    world_r: f32,
    world_b: f32,
    world_t: f32,
}

impl ViewPortFilter {
    fn new(canvas: &CanvasData, screen_rect: egui::Rect) -> Self {
        let top_left = canvas.screen_to_world(screen_rect.left_top().to_vec2());
        let bot_right = canvas.screen_to_world(screen_rect.right_bottom().to_vec2());
        Self {
            world_l: top_left.x,
            world_r: bot_right.x,
            world_t: top_left.y,
            world_b: bot_right.y,
        }
    }

    fn town_in_viewport(&self, town: &Town) -> bool {
        self.world_l < town.x
            && town.x < self.world_r
            && self.world_t < town.y
            && town.y < self.world_b
    }

    fn x_in_viewport(&self, x: f32) -> bool {
        self.world_l < x && x < self.world_r
    }

    fn y_in_viewport(&self, y: f32) -> bool {
        self.world_t < y && y < self.world_b
    }
}
