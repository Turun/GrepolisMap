mod data;
pub(crate) mod state;

use std::collections::linked_list::IterMut;
use std::sync::Arc;

use egui::Stroke;
use std::sync::mpsc;

use crate::message::{MessageToModel, MessageToView, Server, TownSelection};
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
            ui_state: State::Uninitialized,
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

    fn ui_uninitialized(&mut self, ctx: &egui::Context) {
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
                let (response, painter) = ui.allocate_painter(
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

                println!(
                    "{:?}",
                    canvas_data.screen_to_world(
                        response
                            .hover_pos()
                            .or(Some(egui::pos2(0.0, 0.0)))
                            .unwrap()
                            .to_vec2()
                    )
                );

                // DRAW TOWNS
                // towns have a diameter of .25 units, approximately
                let top_left = canvas_data.screen_to_world(response.rect.left_top().to_vec2());
                let bot_right = canvas_data.screen_to_world(response.rect.right_bottom().to_vec2());
                let left = top_left.y;
                let right = bot_right.y;
                let top = top_left.x;
                let bottom = bot_right.x;
                for town in self.ui_data.towns_all.iter().filter(|town| {
                    left < (town.x as f32)
                        && (town.x as f32) < right
                        && top < (town.y as f32)
                        && (town.y as f32) < bottom
                }) {
                    painter.circle_filled(
                        canvas_data
                            .world_to_screen(egui::vec2(town.y as f32, town.x as f32))
                            .to_pos2(),
                        2.0,
                        egui::Color32::from_rgb(25, 200, 100),
                    );
                }

                response
            })
        });
    }
}

impl eframe::App for View {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if let Ok(message) = self.channel_presenter_rx.try_recv() {
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
            }
        }
        let state = &self.ui_state;
        match state {
            State::Uninitialized => self.ui_uninitialized(ctx),
            State::Show(_) => self.ui_init(ctx),
        }
    }
}
