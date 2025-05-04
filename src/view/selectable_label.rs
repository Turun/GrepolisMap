// copied from egui source 0.22 as of 2023-06-30

use egui::{
    Align, NumExt, Response, Sense, TextStyle, Ui, Widget, WidgetInfo, WidgetText, WidgetType,
};

/// One out of several alternatives, either selected or not.
/// Will mark selected items with a different background color.
/// An alternative to [`RadioButton`] and [`Checkbox`].
///
/// Usually you'd use [`Ui::selectable_value`] or [`Ui::selectable_label`] instead.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// #[derive(PartialEq)]
/// enum Enum { First, Second, Third }
/// let mut my_enum = Enum::First;
///
/// ui.selectable_value(&mut my_enum, Enum::First, "First");
///
/// // is equivalent to:
///
/// if ui.add(egui::SelectableLabel::new(my_enum == Enum::First, "First")).clicked() {
///     my_enum = Enum::First
/// }
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct SelectableLabel {
    selected: bool,
    text: WidgetText,
}

impl SelectableLabel {
    pub fn new(selected: bool, text: impl Into<WidgetText>) -> Self {
        Self {
            selected,
            text: text.into(),
        }
    }
}

impl Widget for SelectableLabel {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self { selected, text } = self;

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;

        let wrap_width = ui.available_width() - total_extra.x;
        let text = text.into_galley(ui, None, wrap_width, TextStyle::Button);

        let mut desired_size = total_extra + text.size();

        // my addition
        desired_size.x = desired_size.x.at_least(wrap_width + button_padding.x);
        //

        desired_size.y = desired_size.y.at_least(ui.spacing().interact_size.y);
        let (rect, response) = ui.allocate_at_least(desired_size, Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, text.text())
        });

        if ui.is_rect_visible(response.rect) {
            // my addition, pin the alignment
            let mut layout = *ui.layout();
            layout.main_align = Align::LEFT;
            let text_pos = layout
                .align_size_within_rect(text.size(), rect.shrink2(button_padding))
                .min;
            //

            let visuals = ui.style().interact_selectable(&response, selected);

            if selected || response.hovered() || response.highlighted() || response.has_focus() {
                let rect = rect.expand(visuals.expansion);

                ui.painter().rect(
                    rect,
                    visuals.rounding,
                    visuals.weak_bg_fill,
                    visuals.bg_stroke,
                );
            }

            ui.painter().galley(text_pos, text, visuals.text_color());
        }

        response
    }
}
