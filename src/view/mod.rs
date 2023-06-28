mod data;
pub(crate) mod state;

use std::sync::Arc;

use egui::Stroke;
use std::sync::mpsc;

use crate::message::{MessageToModel, MessageToView, Server};
use crate::view::data::Data;
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
                let size = response.rect;

                // cities have a diameter of .25 units, approximately
                painter.circle_filled(
                    self.ui_data.canvas.center,
                    self.ui_data.canvas.zoom.x,
                    egui::Color32::from_rgb(25, 200, 100),
                );
                self.ui_data.canvas.center += response.drag_delta();
                let scroll_delta = ctx.input(|input| input.scroll_delta.y);
                self.ui_data.canvas.zoom += egui::vec2(scroll_delta, scroll_delta);

                println!("{:#?}", self.ui_data.canvas);

                response
            })
        });
    }
}

impl eframe::App for View {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let state = &self.ui_state;
        match state {
            State::Uninitialized => self.ui_uninitialized(ctx),
            State::Show(_) => self.ui_init(ctx),
        }
    }
}
