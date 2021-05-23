use iced::{Element, Sandbox};

struct App {}

impl Sandbox for App {
    type Message = ();

    fn new() -> Self {
        App {}
    }

    fn title(&self) -> String {
        String::from("Hello world")
    }

    fn update(&mut self, message: Self::Message) {}

    fn view(&mut self) -> Element<'_, Self::Message> {
        iced::Text::new("Hello, world!").into()
    }
}

fn main() {
    App::run(iced::Settings::default());
}
