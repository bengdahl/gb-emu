use iced::{Application, Element};

struct App {
    _gameboy: gb_core::gameboy::Gameboy<gb_core::gameboy::models::DMG>,
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Flags = ();
    type Message = ();

    fn new(_flags: ()) -> (Self, iced::Command<()>) {
        static BLARGG_TEST_ROM: &[u8] = include_bytes!("cpu_instrs.gb");

        let app = App {
            _gameboy: gb_core::gameboy::Gameboy::new(Vec::from(BLARGG_TEST_ROM)).unwrap(),
        };

        let cmd = iced::Command::none();
        (app, cmd)
    }

    fn title(&self) -> String {
        String::from("Hello world")
    }

    fn update(
        &mut self,
        _message: Self::Message,
        _clip: &mut iced::Clipboard,
    ) -> iced::Command<()> {
        iced::Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        iced::Text::new("Hello, world!").into()
    }
}

fn main() {
    App::run(iced::Settings::default()).unwrap();
}
