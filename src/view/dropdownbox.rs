use egui::{Id, Response, Ui, Widget};
use std::hash::Hash;

/// Dropdown widget
pub struct DropDownBox<
    'a,
    F: FnMut(&mut Ui, &str) -> Response,
    V: AsRef<str>,
    I: Iterator<Item = V>,
> {
    buf: &'a mut String,
    popup_id: Id,
    display: F,
    it: I,
}

impl<'a, F: FnMut(&mut Ui, &str) -> Response, V: AsRef<str>, I: Iterator<Item = V>>
    DropDownBox<'a, F, V, I>
{
    /// Creates new dropdown box.
    pub fn from_iter(
        it: impl IntoIterator<IntoIter = I>,
        id_source: impl Hash,
        buf: &'a mut String,
        display: F,
    ) -> Self {
        Self {
            popup_id: Id::new(id_source),
            it: it.into_iter(),
            display,
            buf,
        }
    }
}

impl<'a, F: FnMut(&mut Ui, &str) -> Response, V: AsRef<str>, I: Iterator<Item = V>> Widget
    for DropDownBox<'a, F, V, I>
{
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            popup_id,
            buf,
            it,
            mut display,
        } = self;

        let mut r = ui.text_edit_singleline(buf);
        if r.gained_focus() {
            ui.memory_mut(|m| m.open_popup(popup_id));
        }

        let mut changed = false;
        egui::popup_below_widget(ui, popup_id, &r, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // TODO highlight matching part

                if buf.is_empty() {
                    // show all options
                    for var in it {
                        let text = var.as_ref();
                        if display(ui, text).clicked() {
                            *buf = text.to_owned();
                            changed = true;
                            ui.memory_mut(|m| m.close_popup());
                        }
                    }
                } else {
                    let it = it.collect::<Vec<V>>();
                    let buf_match = buf.clone();
                    if buf.to_lowercase().as_str() == buf.as_str() {
                        // only lowercase input - filter case insensitively
                        let matches_start =
                            |var: &&V| var.as_ref().to_lowercase().starts_with(&*buf_match);
                        let matches_anywhere =
                            |var: &&V| var.as_ref().to_lowercase().contains(&*buf_match);

                        for var in it.iter().filter(matches_start) {
                            let text = var.as_ref();
                            if display(ui, text).clicked() {
                                *buf = text.to_owned();
                                changed = true;
                                ui.memory_mut(|m| m.close_popup());
                            }
                        }
                        for var in it
                            .iter()
                            .filter(|v| matches_anywhere(v) && !matches_start(v))
                        {
                            let text = var.as_ref();
                            if display(ui, text).clicked() {
                                *buf = text.to_owned();
                                changed = true;
                                ui.memory_mut(|m| m.close_popup());
                            }
                        }
                    } else {
                        // mixed case input - filter case sensitively
                        let matches_start = |var: &&V| var.as_ref().starts_with(&*buf_match);
                        let matches_anywhere = |var: &&V| var.as_ref().contains(&*buf_match);

                        for var in it.iter().filter(matches_start) {
                            let text = var.as_ref();
                            if display(ui, text).clicked() {
                                *buf = text.to_owned();
                                changed = true;
                                ui.memory_mut(|m| m.close_popup());
                            }
                        }
                        for var in it
                            .iter()
                            .filter(|v| matches_anywhere(v) && !matches_start(v))
                        {
                            let text = var.as_ref();
                            if display(ui, text).clicked() {
                                *buf = text.to_owned();
                                changed = true;
                                ui.memory_mut(|m| m.close_popup());
                            }
                        }
                    }
                }
            });
        });

        if changed {
            r.mark_changed();
        }

        r
    }
}
