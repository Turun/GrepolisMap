use egui::{text::LayoutJob, Id, Response, TextFormat, Ui, Widget};
use egui_extras::{Column, TableBuilder};

use std::{hash::Hash, sync::Arc};

use super::selectable_label::SelectableLabel;

/// Dropdown widget
pub struct DropDownBox<'a> {
    buf: &'a mut String,
    popup_id: Id,
    opt_it: Option<&'a Arc<Vec<String>>>,
}

impl<'a> DropDownBox<'a> {
    /// Creates new dropdown box.
    pub fn from_iter(
        opt_it: Option<&'a Arc<Vec<String>>>,
        id_source: impl Hash,
        buf: &'a mut String,
    ) -> Self {
        Self {
            popup_id: Id::new(id_source),
            opt_it,
            buf,
        }
    }
}

impl Widget for DropDownBox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            popup_id,
            buf,
            opt_it,
        } = self;

        let mut r = ui.text_edit_singleline(buf);
        if r.gained_focus() {
            ui.memory_mut(|m| m.open_popup(popup_id));
        }

        if let Some(it) = opt_it {
            let mut changed = false;
            egui::popup_below_widget(ui, popup_id, &r, |ui| {
                // first we need to build the list of displayed options. first come entries
                // where the beginning matches, second the one where the match is anywhere in the string.
                // an elegant, iterator based solution is spoiled by the borrow checker. So we just have
                // a giant for loop to prepare the table entries
                let mut first = Vec::new();
                let mut second = Vec::new();
                let emphasize = egui::TextFormat {
                    color: ui.style().visuals.warn_fg_color,
                    ..Default::default()
                };

                for var in &**it {
                    let s = var.as_ref();
                    if buf.is_empty() {
                        let mut job = LayoutJob::default();
                        job.append(s, 0.0, TextFormat::default());
                        first.push((s.to_string(), job));
                        continue;
                    }

                    // buf is now guaranteed to not be empty
                    let lower_s = s.to_lowercase();
                    let mat = if buf.to_lowercase().as_str() == buf.as_str() {
                        // input is all lowercase -> match case insensitive
                        lower_s.match_indices(&*buf).collect::<Vec<(usize, &str)>>()
                    } else {
                        s.match_indices(&*buf).collect::<Vec<(usize, &str)>>()
                    };

                    if mat.is_empty() {
                        // not a match
                        continue;
                    }

                    if mat[0].0 == 0 {
                        // matching the start of the string
                        let mut job = LayoutJob::default();
                        let mut cursor = mat[0].0 + mat[0].1.len();
                        job.append(&s[0..cursor], 0.0, emphasize.clone());
                        for (index, text) in mat.iter().skip(1) {
                            job.append(&s[cursor..*index], 0.0, TextFormat::default());
                            cursor = index + text.len();
                            job.append(&s[*index..cursor], 0.0, emphasize.clone());
                        }
                        job.append(&s[cursor..], 0.0, TextFormat::default());
                        first.push((s.to_string(), job));
                    } else {
                        // matching somewhere in the string, but not the start
                        let mut cursor = 0;
                        let mut job = LayoutJob::default();
                        for (index, text) in &mat {
                            job.append(&s[cursor..*index], 0.0, TextFormat::default());
                            cursor = index + text.len();
                            job.append(&s[*index..cursor], 0.0, emphasize.clone());
                        }
                        job.append(&s[cursor..], 0.0, TextFormat::default());
                        second.push((s.to_string(), job));
                    }
                }
                let combined = [first, second].concat();

                let _r = egui::ScrollArea::vertical().show(ui, |ui| {
                    let text_height = egui::TextStyle::Body.resolve(ui.style()).size;
                    let table = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(false)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::remainder())
                        .min_scrolled_height(0.0);
                    table.body(|body| {
                        body.rows(text_height, combined.len(), |mut row| {
                            let row_index = row.index();
                            row.col(|ui| {
                                let (text, layoutjob) = combined[row_index].clone();
                                let label = SelectableLabel::new(false, layoutjob).ui(ui);
                                if label.clicked() {
                                    *buf = text;
                                    changed = true;
                                    ui.memory_mut(egui::Memory::close_popup);
                                }
                                if label.has_focus() {
                                    // tell the global response that we have focus
                                }
                            });
                        });
                    });
                });
            });

            if changed {
                r.mark_changed();
            }
        }

        r
    }
}
