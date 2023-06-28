use crate::model::Model;
use crate::view::View;
use tokio::sync::mpsc;

pub struct Presenter {
    model: Model,
    view: View,
    channel_tx: mpsc::UnboundedSender<f32>,
    channel_rx: mpsc::UnboundedReceiver<f32>,
}

impl Presenter {
    pub fn new() -> Self {
        let (view_rx, self_tx) = mspc::channel_unbounded();
        let (self_rx, view_tx) = mspc::channel_unbounded();

        Self {
            model: Model::new(),
            view: View::new(),
            channel_tx: self_tx,
            channel_rx: self_rx,
        }
    }

    pub fn start(self) -> Result<(), ()> {
        /*   4   │
           5   │ fn main() -> Result<(), eframe::Error> {
           6   │     println!("Hello World");
           7   │     eframe::run_native(
           8   │         "TEST WINDOW",
           9   │         eframe::NativeOptions::default(),
          10   │         Box::new(|cc| Box::new(Canvas::new(cc))),
          11   │     )
          12   │ }
          13   │
          14   │ struct Canvas {
          15   │     lines: Vec<egui::Pos2>,
          16   │     stroke: egui::Stroke,
          17   │ }
          18   │
          19   │ impl Default for Canvas {
          20   │     fn default() -> Self {
          21   │         Self {
          22   │             lines: Default::default(),
          23   │             stroke: egui::Stroke::new(2.0, egui::Color32::from_rgb(25, 200, 100)),
          24   │         }
          25   │     }
          26   │ }
          27   │
          28   │ impl Canvas {
          29   │     pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
          30   │         Default::default()
          31   │     }
          32   │ }
        */

        Ok(())
    }
}
