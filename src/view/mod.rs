mod data;
mod dropdownbox;
pub(crate) mod state;

use std::sync::mpsc;

use egui::{ProgressBar, Shape, Ui};

use crate::message::{
    FromType, MessageToModel, MessageToView, Progress, Server, Town, TownConstraint, TownSelection,
};
use crate::view::data::{CanvasData, Data, ViewPortFilter};
use crate::view::dropdownbox::DropDownBox;
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

    fn ui_server_input(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Server ID");
            ui.text_edit_singleline(&mut self.ui_data.server_id);
        });
        if ui
            .add(egui::Button::new("Load Data for this Server"))
            .clicked()
        {
            self.ui_state = State::Uninitialized(Progress::None);
            self.channel_presenter_tx
                .send(MessageToModel::SetServer(Server {
                    id: self.ui_data.server_id.clone(),
                }))
                .expect("Failed to send the SetServer Message to the backend");
        }
    }

    fn ui_uninitialized(&mut self, ctx: &egui::Context, progress: Progress) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                self.ui_server_input(ui);
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
                self.ui_server_input(ui);
                ui.label(format!("Towns Total: {}", self.ui_data.all_towns.len()));
                ui.label(format!(
                    "Towns Selected: {}",
                    self.ui_data.ghost_towns.len()
                ));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.ui_data.settings_all.enabled, "");
                    ui.label("All Towns:");
                    ui.color_edit_button_srgba(&mut self.ui_data.settings_all.color);
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.ui_data.settings_ghosts.enabled, "");
                    ui.label("Ghost Towns:");
                    ui.color_edit_button_srgba(&mut self.ui_data.settings_ghosts.color);
                });
                ui.separator();
                if ui.button("Add Towns").clicked() {
                    self.ui_data.selections.push(TownConstraint {
                        from_type: FromType::Player,
                        color: egui::Color32::GREEN,
                        value: String::from(""),
                        towns: Vec::new(),
                    })
                }
                ui.separator();
                // TODO: figure out how we can fetch the data corresponding to each selection. We have to send a new
                // request every time the selection values change. When that happens we must send a request to the backend. The message
                // ingest loop will have to track all incoming updates and update the value of the correct selection accordingly
                for (index, selection) in self.ui_data.selections.iter_mut().enumerate() {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.selectable_value(
                                &mut selection.from_type,
                                FromType::Player,
                                "Player",
                            );
                            ui.selectable_value(
                                &mut selection.from_type,
                                FromType::Alliance,
                                "Alliance",
                            );
                        });
                        ui.add(DropDownBox::from_iter(
                            match selection.from_type {
                                FromType::Player => &mut self.ui_data.name_players,
                                FromType::Alliance => &mut self.ui_data.name_alliances,
                            },
                            format!("Selection {}", index),
                            &mut selection.value,
                            |ui, text| ui.selectable_label(false, text),
                        ))
                    });
                }

                let ddb = DropDownBox::from_iter(
                    self.ui_data
                        .ghost_towns
                        .iter()
                        .map(|town| town.name.to_owned())
                        .collect::<Vec<String>>(),
                    "dropdown box",
                    &mut self.ui_data.drop_down_string,
                    |ui, text| ui.selectable_label(false, text),
                );
                ui.add(ddb);
            });
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
                    .all_towns
                    .iter()
                    .filter(|town| filter.town_in_viewport(town))
                    .collect();
                let visible_ghost_towns: Vec<&Town> = self
                    .ui_data
                    .ghost_towns
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
                if self.ui_data.settings_all.enabled {
                    for town in &visible_towns_all {
                        painter.circle_filled(
                            canvas_data
                                .world_to_screen(egui::vec2(town.x, town.y))
                                .to_pos2(),
                            1.0 + canvas_data.scale_world_to_screen(0.15),
                            if town.player_id == Some(1495649) {
                                egui::Color32::WHITE
                            } else {
                                self.ui_data.settings_all.color
                            },
                        );
                    }
                }

                // DRAW GHOST TOWNS
                if self.ui_data.settings_ghosts.enabled {
                    for town in &visible_ghost_towns {
                        painter.circle_filled(
                            canvas_data
                                .world_to_screen(egui::vec2(town.x, town.y))
                                .to_pos2(),
                            2.0 + canvas_data.scale_world_to_screen(0.15),
                            self.ui_data.settings_ghosts.color,
                        );
                    }
                }

                // DRAW SELECTED TOWS
                for selection in &self.ui_data.selections {
                    for town in &selection.towns {
                        painter.circle_filled(
                            canvas_data
                                .world_to_screen(egui::vec2(town.x, town.y))
                                .to_pos2(),
                            1.0 + canvas_data.scale_world_to_screen(0.15),
                            selection.color,
                        );
                    }
                }

                // POPUP WITH TOWN INFORMATION
                // TODO more information (hydrate our town structs more)
                if canvas_data.zoom > 10.0 {
                    let optional_mouse_position = response.hover_pos();
                    response = response.on_hover_ui_at_pointer(|ui| {
                        let position = if let Some(mouse_position) = optional_mouse_position {
                            canvas_data
                                .screen_to_world(mouse_position.to_vec2())
                                .to_pos2()
                        } else {
                            return;
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
                MessageToView::GotServer(all_towns, player_names, alliance_names) => {
                    self.ui_state = State::Show(TownSelection::None);
                    self.ui_data.all_towns = all_towns;
                    self.ui_data.name_players = player_names;
                    self.ui_data.name_alliances = alliance_names;
                    self.channel_presenter_tx
                        .send(MessageToModel::FetchGhosts)
                        .expect("Failed to send message to model: TownSelection::Ghosts");
                }
                MessageToView::TownList(constraint, town_list) => {
                    self.ui_state = State::Show(TownSelection::Ghosts);
                    self.ui_data.ghost_towns = town_list;
                    // TODO don't assign the value to the ghost towns, assign it to the correct user selection
                }
                MessageToView::GhostTowns(town_list) => {
                    self.ui_data.ghost_towns = town_list;
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
