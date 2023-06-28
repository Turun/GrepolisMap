use rand;
use eframe;
use egui;

fn main() -> Result<(), eframe::Error> {
    println!("Hello World");
    eframe::run_native(
        "TEST WINDOW",
        eframe::NativeOptions::default(),
        Box::new(|cc| Box::new(Canvas::new(cc))),
    )
}

struct Canvas {
    lines: Vec<egui::Pos2>,
    stroke: egui::Stroke,
}

impl Default for Canvas {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            stroke: egui::Stroke::new(2.0, egui::Color32::from_rgb(25, 200, 100)),
        }
    }
}

impl Canvas {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }
}

impl eframe::App for Canvas {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        egui::CentralPanel::default().show(ctx, |ui| 
            // ui.button("click me!")
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                let (mut response, painter) = ui.allocate_painter(
                    ui.available_size_before_wrap(), egui::Sense::drag()
                );

                self.lines.insert(0, egui::pos2(rand::random(), rand::random()));
                self.lines.insert(0, egui::pos2(rand::random(), rand::random()));

                while self.lines.len() > 20 {
                    self.lines.pop();
                }

                // todo: user interaction
                // line coordinates are in [0,1), but canvas work with pixels.
                let to_screen = eframe::emath::RectTransform::from_to(
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::Vec2{x: 1.0, y: 1.0}),
                    response.rect
                );
                let transformed_lines = self.lines.iter().map(|p| to_screen * *p).collect();
                // let transformed_lines = self.lines.iter().map(|p| egui::pos2(p.x * 200.0, p.y * 200.0)).collect();
                let shape = egui::Shape::line(transformed_lines, self.stroke);
                painter.add(shape);

                response
            })
        );
    }
}
