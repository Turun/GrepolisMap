use iced::executor;
use iced::widget::canvas;
use iced::widget::container;
use iced::{
    Application, Color, Command, Element, Length, Point, Rectangle, Renderer, Settings, Size, Theme,
};

pub fn main() -> iced::Result {
    Presenter::run(Settings {
        antialiasing: true,
        ..Settings::default()
    })
}

struct Presenter {
    view: View,
    model: Model,
}

struct View {}

struct Model {
    cities: Vec<(f32, f32)>,
}

impl Model {
    fn new() -> Self {
        Self {
            cities: (0..20).map(|i| (rand::random(), rand::random())).collect(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Update,
}

impl Application for Presenter {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                view: View {},
                model: Model::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Test")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let my_canvas = canvas(self as &Self)
            .width(Length::Fill)
            .height(Length::Fill);

        container(my_canvas)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(20)
            .into()
    }
}

impl<Message> canvas::Program<Message, Renderer> for Presenter {
    type State = ();

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        // theme: &Theme,
        bounds: Rectangle,
        cursor: canvas::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(Size::new(bounds.width, bounds.height));
        for (left, right) in self
            .model
            .cities
            .iter()
            .skip(1)
            .zip(self.model.cities.iter())
        {
            let line = canvas::Path::line(
                Point {
                    x: left.0,
                    y: left.1,
                },
                Point {
                    x: right.0,
                    y: right.1,
                },
            );
            frame.stroke(
                &line,
                canvas::Stroke::default()
                    .with_color(Color::from_rgb(1.0, 0.1, 0.1))
                    .with_width(3.0),
            );
        }
        vec![frame.into_geometry()]
    }
}
