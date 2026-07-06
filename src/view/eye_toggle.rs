use egui::{Response, Sense, Shape, Stroke, Ui, WidgetInfo, WidgetType};

/// A small eye icon button that toggles `*visible`. Draws an open eye when `*visible`
/// is true, and the same eye with a diagonal line through it ("crossed out") otherwise.
///
/// The icon is hand-drawn with the painter (rather than a font glyph) because the
/// bundled egui/NotoSansJP fonts don't include an eye emoji glyph.
pub fn eye_toggle(ui: &mut Ui, visible: &mut bool) -> Response {
    let desired_size = egui::vec2(20.0, 16.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click());

    if response.clicked() {
        *visible = !*visible;
        response.mark_changed();
    }
    response.widget_info(|| {
        WidgetInfo::selected(
            WidgetType::Checkbox,
            *visible,
            if *visible { "visible" } else { "hidden" },
        )
    });

    if ui.is_rect_visible(rect) {
        let visuals = ui.style().interact_selectable(&response, *visible);
        let stroke_color = visuals.fg_stroke.color;

        let center = rect.center();
        let eye_w = rect.width() * 0.9;
        let eye_h = rect.height() * 0.65;

        // sample points along an ellipse to approximate the eye's outline
        const SEGMENTS: usize = 24;
        let outline: Vec<egui::Pos2> = (0..SEGMENTS)
            .map(|i| {
                let t = i as f32 / SEGMENTS as f32 * std::f32::consts::TAU;
                egui::pos2(
                    center.x + (eye_w / 2.0) * t.cos(),
                    center.y + (eye_h / 2.0) * t.sin(),
                )
            })
            .collect();
        let painter = ui.painter();
        painter.add(Shape::closed_line(outline, Stroke::new(1.5, stroke_color)));
        painter.circle_filled(center, eye_h * 0.22, stroke_color);

        if !*visible {
            // diagonal slash across the eye to indicate it is toggled off
            let half_diagonal = egui::vec2(eye_w, eye_h) * 0.5;
            painter.line_segment(
                [center - half_diagonal, center + half_diagonal],
                Stroke::new(1.5, stroke_color),
            );
        }
    }

    response
}
