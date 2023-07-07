mod data;
mod dropdownbox;
mod selectable_label;
pub(crate) mod state;
use std::sync::{mpsc, Arc};
use std::time::Duration;
use strum::IntoEnumIterator;

use egui::{FontData, ProgressBar, Shape, Ui};

use crate::message::{MessageToModel, MessageToView, Progress, Server};
use crate::towns::{
    Change, Comparator, Constraint, ConstraintType, SelectionState, Town, TownSelection,
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

    fn setup(self, cc: &eframe::CreationContext) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            String::from("Custom Font"),
            FontData::from_static(include_bytes!("../../NotoSansJP-Regular.ttf")),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .push(String::from("Custom Font"));
        cc.egui_ctx.set_fonts(fonts);

        self
    }

    pub fn start(self) {
        let native_options = eframe::NativeOptions::default();
        // TODO Save config between app runs.
        let _result = eframe::run_native(
            "Grepolis Map",
            native_options,
            Box::new(|cc| Box::new(self.setup(cc))),
        );
    }

    fn ui_server_input(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let mut should_load_server = false;
        ui.horizontal(|ui| {
            ui.label("Server ID");
            let response = ui.text_edit_singleline(&mut self.ui_data.server_id);
            if response.lost_focus()
                && response
                    .ctx
                    .input(|input| input.key_pressed(egui::Key::Enter))
            {
                // detect enter on text field: https://github.com/emilk/egui/issues/229
                should_load_server = true;
            }
        });
        if ui
            .add(egui::Button::new("Load Data for this Server"))
            .clicked()
        {
            should_load_server = true;
        }

        if should_load_server {
            self.ui_state = State::Uninitialized(Progress::None);
            self.ui_data = Data {
                server_id: self.ui_data.server_id.clone(),
                canvas: Option::default(),
                settings_all: self.ui_data.settings_all.clone(),
                settings_ghosts: self.ui_data.settings_ghosts.clone(),
                selections: self.ui_data.selections.clone(),
                all_towns: Arc::new(Vec::new()),
                ghost_towns: Arc::new(Vec::new()),
            };
            // ensure the towns in the selection are fetched anew after loading the data from the server.
            // If we don't do this the selection may become stale and show towns from server ab12 on a
            // map that is otherwise pulled from server cd34
            for selection in &mut self.ui_data.selections {
                selection.state = SelectionState::NewlyCreated;
            }
            self.channel_presenter_tx
                .send(MessageToModel::SetServer(
                    Server {
                        id: self.ui_data.server_id.clone(),
                    },
                    ctx.clone(),
                ))
                .expect("Failed to send the SetServer Message to the backend");
        }
    }

    fn ui_uninitialized(&mut self, ctx: &egui::Context, progress: Progress) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                self.ui_server_input(ui, ctx);
                match progress {
                    Progress::None => {}
                    Progress::Started => {
                        ui.add(ProgressBar::new(0.0).text(format!("{progress:?}")));
                    }
                    Progress::IslandOffsets => {
                        ui.add(ProgressBar::new(0.2).text(format!("{progress:?}")));
                    }
                    Progress::Alliances => {
                        ui.add(ProgressBar::new(0.4).text(format!("{progress:?}")));
                    }
                    Progress::Players => {
                        ui.add(ProgressBar::new(0.6).text(format!("{progress:?}")));
                    }
                    Progress::Towns => {
                        ui.add(ProgressBar::new(0.8).text(format!("{progress:?}")));
                    }
                    Progress::Islands => {
                        ui.add(ProgressBar::new(1.0).text(format!("{progress:?}")));
                    }
                }
            });
        });
    }

    #[allow(clippy::too_many_lines)] // UI Code, am I right, hahah
    fn ui_init(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("left panel").show(ctx, |ui| {
            ui.vertical(|ui| {
                self.ui_server_input(ui, ctx);
                ui.label(format!("Total Towns: {}", self.ui_data.all_towns.len()));
                ui.label(format!("Ghost Towns: {}", self.ui_data.ghost_towns.len()));
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
                let mut change: Option<Change> = None;
                for (index, selection) in self.ui_data.selections.iter_mut().enumerate() {
                    let _first_row_response = ui.horizontal(|ui| {
                        ui.color_edit_button_srgba(&mut selection.color);
                        if ui.button("Add Towns").clicked() {
                            change = Some(Change::Add);
                        }
                        if ui.button("Remove").clicked() {
                            change = Some(Change::Remove(index));
                        }
                        if ui.button("↑").clicked() {
                            change = Some(Change::MoveUp(index));
                        }
                        if ui.button("↓").clicked() {
                            change = Some(Change::MoveDown(index));
                        }

                        ui.label(format!("{} Towns", selection.towns.len()));

                        if selection.state == SelectionState::Loading {
                            ui.spinner();
                        }
                    });

                    let num_constraints = selection.constraints.len();
                    let mut request_update = selection.state == SelectionState::NewlyCreated; // A newly created one must request an update immediately to fill its ddv list
                    let mut constraint_change = None;
                    for (cindex, constraint) in selection.constraints.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            let _inner_response = egui::ComboBox::from_id_source(format!(
                                "ComboxBox {index}/{cindex} Type"
                            ))
                            .width(ui.style().spacing.interact_size.x * 3.5)
                            .selected_text(format!("{}", constraint.constraint_type))
                            .show_ui(ui, |ui| {
                                for value in ConstraintType::iter() {
                                    let text = value.to_string();
                                    if ui
                                        .selectable_value(
                                            &mut constraint.constraint_type,
                                            value,
                                            text,
                                        )
                                        .clicked()
                                    {
                                        request_update = true;
                                    }
                                }
                            });

                            let _inner_response = egui::ComboBox::from_id_source(format!(
                                "ComboxBox {index}/{cindex} Comparator"
                            ))
                            .width(ui.style().spacing.interact_size.x * 1.75)
                            .selected_text(format!("{}", constraint.comparator))
                            .show_ui(ui, |ui| {
                                for value in Comparator::iter() {
                                    let text = value.to_string();
                                    if ui
                                        .selectable_value(&mut constraint.comparator, value, text)
                                        .clicked()
                                    {
                                        request_update = true;
                                    }
                                }
                            });

                            let ddb = DropDownBox::from_iter(
                                constraint.drop_down_values.as_ref(),
                                format!("ComboBox {index}/{cindex} Value"),
                                &mut constraint.value,
                            );
                            if ui
                                .add_sized(
                                    [
                                        ui.style().spacing.interact_size.x * 4.5,
                                        ui.style().spacing.interact_size.y,
                                    ],
                                    ddb,
                                )
                                .changed()
                            {
                                request_update = true;
                            };
                            if cindex + 1 == num_constraints {
                                if ui.button("+").clicked() {
                                    constraint_change = Some(Change::Add);
                                    request_update = true;
                                }
                            } else {
                                ui.label("and");
                            }
                            if ui.button("-").clicked() {
                                constraint_change = Some(Change::Remove(cindex));
                                request_update = true;
                            }
                            if ui.button("↑").clicked() {
                                constraint_change = Some(Change::MoveUp(cindex));
                            }
                            if ui.button("↓").clicked() {
                                constraint_change = Some(Change::MoveDown(cindex));
                            }
                        });
                    }

                    if let Some(change) = constraint_change {
                        match change {
                            Change::MoveUp(index) => {
                                if index >= 1 {
                                    selection.constraints.swap(index, index - 1);
                                }
                            }
                            Change::Remove(index) => {
                                let _element = selection.constraints.remove(index);
                            }
                            Change::MoveDown(index) => {
                                if index + 1 < selection.constraints.len() {
                                    selection.constraints.swap(index, index + 1);
                                }
                            }
                            Change::Add => selection.constraints.push(Constraint::default()),
                        }
                    }

                    if request_update {
                        self.channel_presenter_tx
                            .send(MessageToModel::FetchTowns(selection.partial_clone()))
                            .expect(&format!(
                                "Failed to send Message to Model for Selection {}",
                                &selection
                            ));
                        selection.state = SelectionState::Loading;
                        for constraint in &mut selection.constraints {
                            constraint.drop_down_values = None;
                        }
                    }
                    ui.separator();
                }

                if let Some(change_action) = change {
                    match change_action {
                        Change::MoveUp(index) => {
                            if index >= 1 {
                                self.ui_data.selections.swap(index, index - 1);
                            }
                        }
                        Change::Remove(index) => {
                            let _elem = self.ui_data.selections.remove(index);
                        }
                        Change::MoveDown(index) => {
                            if index + 1 < self.ui_data.selections.len() {
                                self.ui_data.selections.swap(index, index + 1);
                            }
                        }
                        Change::Add => {
                            self.ui_data.selections.push(TownSelection::default());
                        }
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let (mut response, painter) = ui.allocate_painter(
                    ui.available_size_before_wrap(),
                    egui::Sense::click_and_drag(),
                );

                if self.ui_data.canvas.is_none() {
                    self.ui_data.canvas =
                        Some(CanvasData::new(-response.rect.left_top().to_vec2()));
                }
                let canvas_data = self.ui_data.canvas.as_mut().unwrap();

                //DRAG
                canvas_data.world_offset_px -=
                    canvas_data.scale_screen_to_world(response.drag_delta());

                // ZOOM
                // as per https://www.youtube.com/watch?v=ZQ8qtAizis4
                if response.hovered() {
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
                }

                // filter everything that is not visible
                let filter = ViewPortFilter::new(canvas_data, response.rect);
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
                for i in (0u16..=10).map(|i| f32::from(i) * 100.0) {
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
                    for i in (0u16..=100)
                        .map(|i| f32::from(i) * 10.0)
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
                            self.ui_data.settings_all.color,
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
                    for town in selection
                        .towns
                        .iter()
                        .filter(|t| filter.town_in_viewport(t))
                    {
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
                        ui.label(format!("{position:?}"));

                        if !visible_towns_all.is_empty() {
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

                            if closest_distance < 1.5 {
                                ui.label(format!(
                                    "{}\nPoints: {}\nPlayer: {}\nAlliance: {}",
                                    closest_town.name,
                                    closest_town.points,
                                    if let Some(name) = &closest_town.player_name {
                                        name
                                    } else {
                                        ""
                                    },
                                    if let Some(name) = &closest_town.alliance_name {
                                        name
                                    } else {
                                        ""
                                    },
                                ));
                            }
                        }
                    });
                }

                response
            })
        });
    }
}

impl eframe::App for View {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        while let Ok(message) = self.channel_presenter_rx.try_recv() {
            println!("Got Message from Model to View: {message}");
            match message {
                MessageToView::GotServer => {
                    self.ui_state = State::Show;
                    self.channel_presenter_tx
                        .send(MessageToModel::FetchAll)
                        .expect("Failed to send message to model: FetchAll");
                    self.channel_presenter_tx
                        .send(MessageToModel::FetchGhosts)
                        .expect("Failed to send message to model: FetchGhosts");
                }
                MessageToView::TownListForSelection(selection, town_list) => {
                    self.ui_state = State::Show;
                    let optional_selection = self
                        .ui_data
                        .selections
                        .iter_mut()
                        .find(|element| *element == selection);
                    if let Some(selection) = optional_selection {
                        selection.towns = town_list;
                        selection.state = SelectionState::Finished;
                    } else {
                        println!("No existing selection found for {selection}");
                    }
                }
                MessageToView::ValueListForConstraint(constraint, selection, towns) => {
                    self.ui_state = State::Show;
                    let optional_selection = self
                        .ui_data
                        .selections
                        .iter_mut()
                        .find(|element| *element == selection);
                    if let Some(selection) = optional_selection {
                        let optional_constraint =
                            selection.constraints.iter_mut().find(|c| **c == constraint);
                        if let Some(constraint) = optional_constraint {
                            constraint.drop_down_values = Some(towns);
                        } else {
                            println!(
                                "No existing constraint {constraint} found in selection {selection}"
                            );
                        }
                    } else {
                        println!("No existing selection found for {selection}");
                    }
                }
                MessageToView::AllTowns(towns) => {
                    self.ui_state = State::Show;
                    self.ui_data.all_towns = towns;
                }
                MessageToView::GhostTowns(towns) => {
                    self.ui_state = State::Show;
                    self.ui_data.ghost_towns = towns;
                }
                MessageToView::Loading(progress) => {
                    self.ui_state = State::Uninitialized(progress);
                }
            }
        }
        let state = self.ui_state.clone();
        match state {
            State::Uninitialized(progress) => self.ui_uninitialized(ctx, progress),
            State::Show => self.ui_init(ctx),
        }

        // allow the user to zoom in and out
        // https://docs.rs/egui/latest/egui/gui_zoom/fn.zoom_with_keyboard_shortcuts.html
        if !frame.is_web() {
            egui::gui_zoom::zoom_with_keyboard_shortcuts(ctx, frame.info().native_pixels_per_point);
        }

        // make sure we process messages from the backend every once in a while
        ctx.request_repaint_after(Duration::from_millis(500));
    }
}
