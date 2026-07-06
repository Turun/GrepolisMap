use egui::Shape;

use crate::town::Town;

use super::{
    data::{CanvasData, ViewPortFilter},
    View,
};

/// brand's lightish green (sampled from assets/grass_touchers_logo.png), used to
/// highlight the ctrl+drag rectangle selection and the towns it picks out.
const SELECTION_GREEN: egui::Color32 = egui::Color32::from_rgb(143, 199, 62);

impl View {
    #[allow(clippy::too_many_lines)] // UI Code, am I right, hahah
    pub fn ui_map(&mut self, ctx: &egui::Context) {
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
                // we need to have this as an option so we are reminded when we have to
                // reset it. The .unwrap here is fine, because if it is none we make it
                // Some() just a line above this comment.
                let canvas_data = self.ui_data.canvas.as_mut().unwrap();

                let ctrl_held = ctx.input(|input| input.modifiers.ctrl);

                //DRAG (panning). Ctrl+drag is reserved for the rectangle town selection below.
                if !ctrl_held {
                    canvas_data.world_offset_px -=
                        canvas_data.scale_screen_to_world(response.drag_delta());
                }

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

                    // since egui 0.26 there is the option of smooth scrolling. I don't think it'll be an improvement though
                    let scroll_delta = ctx.input(|input| input.raw_scroll_delta.y);
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

                // every town that is *actually drawn as a dot* right now: respects the "all
                // towns"/"ghost towns" toggles, the viewport, and includes towns that are only
                // shown because they match a (non-transparent) named/colored selection. This is
                // the set the ctrl+drag rectangle below is allowed to pick from, so the existing
                // filters can be used to narrow down what's selectable before marquee-selecting.
                let rendered_towns: Vec<&Town> = {
                    let mut re: Vec<&Town> = Vec::new();
                    if self.ui_data.settings_all.enabled {
                        re.extend(visible_towns_all.iter().copied());
                    }
                    if self.ui_data.settings_ghosts.enabled {
                        re.extend(visible_ghost_towns.iter().copied());
                    }
                    for selection in &self.ui_data.selections {
                        if selection.color.a() == 0 {
                            continue;
                        }
                        re.extend(
                            selection
                                .towns
                                .iter()
                                .filter(|t| filter.town_in_viewport(t)),
                        );
                    }
                    re
                };

                // CTRL+DRAG RECTANGLE TOWN SELECTION
                if ctrl_held {
                    if response.drag_started() {
                        self.marquee_drag_start = response.interact_pointer_pos();
                    }
                } else {
                    self.marquee_drag_start = None;
                }
                let marquee_rect = ctrl_held.then(|| self.marquee_drag_start).flatten().and_then(
                    |start| {
                        response
                            .interact_pointer_pos()
                            .or_else(|| response.hover_pos())
                            .map(|current| egui::Rect::from_two_pos(start, current))
                    },
                );
                if let Some(rect) = marquee_rect {
                    if response.drag_released() {
                        self.selected_town_ids = rendered_towns
                            .iter()
                            .filter(|town| {
                                rect.contains(
                                    canvas_data.world_to_screen(egui::vec2(town.x, town.y)).to_pos2(),
                                )
                            })
                            .map(|town| town.id)
                            .collect();
                        self.marquee_drag_start = None;
                    }
                }

                // CLICK HANDLING. `clicked()` never fires for the drag that creates a new
                // marquee rectangle above, so neither of these can undo it.
                if response.clicked() {
                    if ctrl_held {
                        // ctrl+click (no drag) toggles just the nearest rendered town under the
                        // cursor in/out of the selection, leaving the rest of it untouched.
                        if let Some(click_pos) = response.interact_pointer_pos() {
                            let closest = rendered_towns.iter().min_by(|a, b| {
                                let da = canvas_data
                                    .world_to_screen(egui::vec2(a.x, a.y))
                                    .to_pos2()
                                    .distance(click_pos);
                                let db = canvas_data
                                    .world_to_screen(egui::vec2(b.x, b.y))
                                    .to_pos2()
                                    .distance(click_pos);
                                da.total_cmp(&db)
                            });
                            if let Some(town) = closest {
                                let screen_pos = canvas_data
                                    .world_to_screen(egui::vec2(town.x, town.y))
                                    .to_pos2();
                                // match the actual drawn dot radius (see the "DRAW ALL/GHOST
                                // TOWNS" sections below) plus a small constant margin, so the
                                // clickable area always covers the visible dot instead of a
                                // fixed pixel radius that falls inside it once zoomed in.
                                let hit_radius_px = 4.0 + canvas_data.scale_world_to_screen(0.15);
                                if screen_pos.distance(click_pos) <= hit_radius_px
                                    && !self.selected_town_ids.remove(&town.id)
                                {
                                    self.selected_town_ids.insert(town.id);
                                }
                            }
                        }
                    } else {
                        // a plain click (no ctrl) clears the whole selection
                        self.selected_town_ids.clear();
                    }
                }
                if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
                    self.selected_town_ids.clear();
                }

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
                    // if this selection if made fully transparent, skip the work
                    if selection.color.a() == 0 {
                        continue;
                    }

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

                // DRAW MARQUEE (CTRL+DRAG) SELECTED TOWNS as a highlighted ring
                if !self.selected_town_ids.is_empty() {
                    for town in rendered_towns
                        .iter()
                        .filter(|t| self.selected_town_ids.contains(&t.id))
                    {
                        painter.circle_stroke(
                            canvas_data
                                .world_to_screen(egui::vec2(town.x, town.y))
                                .to_pos2(),
                            3.0 + canvas_data.scale_world_to_screen(0.15),
                            egui::Stroke::new(2.0, SELECTION_GREEN),
                        );
                    }
                }

                // DRAW THE LIVE CTRL+DRAG SELECTION RECTANGLE
                if let Some(rect) = marquee_rect {
                    painter.rect_filled(
                        rect,
                        0.0,
                        egui::Color32::from_rgba_unmultiplied(143, 199, 62, 30),
                    );
                    painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, SELECTION_GREEN));
                }

                // RIGHT CLICK MENU FOR ACTIONS ON THE (MARQUEE) SELECTION
                response.context_menu(|ui| {
                    if ui
                        .button(t!("map.context_menu.copy_mailing_list"))
                        .clicked()
                    {
                        // unique, alphabetically sorted owners of the selected towns, joined as
                        // "adamb; bobbydonut; lawnjittle; timbrick". Ghost/unowned towns have no
                        // player and are skipped.
                        let owners: std::collections::BTreeSet<&str> = self
                            .ui_data
                            .all_towns
                            .iter()
                            .chain(self.ui_data.ghost_towns.iter())
                            .filter(|town| self.selected_town_ids.contains(&town.id))
                            .filter_map(|town| town.player_name.as_deref())
                            .collect();
                        let mailing_list = owners.into_iter().collect::<Vec<_>>().join("; ");
                        ctx.copy_text(mailing_list);
                        ui.close_menu();
                    }
                });

                // POPUP WITH TOWN INFORMATION
                if canvas_data.zoom > 10.0 {
                    let optional_mouse_position = response.hover_pos();
                    // NOTE: This is broken with the jump from egui 0.26 to egui 0.28. The hover ui now apparently
                    // remembers the smallest width it ever had and will never go above that width. Since we have short
                    // and long hover texts this causes unnecessary and ugly wrapping.
                    // Solved it by downgrading. But it if I'm felling up to it I might make an issue later.
                    response = response.on_hover_ui_at_pointer(|ui| {
                        let position = if let Some(mouse_position) = optional_mouse_position {
                            canvas_data
                                .screen_to_world(mouse_position.to_vec2())
                                .to_pos2()
                        } else {
                            return;
                        };
                        ui.label(format!("{position:?}"));

                        if visible_towns_all.is_empty() {
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

                        if closest_distance >= 1.5 {
                            return;
                        }
                        ui.label(t!(
                            "map.hover",
                            name = closest_town.name,
                            points = closest_town.points,
                            player = closest_town.player_name.as_deref().unwrap_or(""),
                            alliance = closest_town.alliance_name.as_deref().unwrap_or(""),
                        ));
                    });
                }

                // LOGO OVERLAY, fixed to the top left corner of the map regardless of pan/zoom
                let logo_max_size = egui::vec2(48.0, 48.0);
                let logo_rect = egui::Rect::from_min_size(
                    response.rect.left_top() + egui::vec2(8.0, 8.0),
                    logo_max_size,
                );
                ui.put(
                    logo_rect,
                    egui::Image::new(egui::include_image!(
                        "../../assets/grass_touchers_logo.png"
                    ))
                    .max_size(logo_max_size)
                    .maintain_aspect_ratio(true),
                );

                response
            })
        });
    }
}
