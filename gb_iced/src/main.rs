use std::path::PathBuf;

use iced::{Application, Color, Element, Length, Settings};

struct App {
    gameboy: gb_core::gameboy::Gameboy<gb_core::gameboy::models::DMG>,
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Flags = PathBuf;
    type Message = ();

    fn new(rom_path: PathBuf) -> (Self, iced::Command<()>) {
        use std::io::Read;
        let mut rom = std::fs::File::open(rom_path).unwrap();
        let mut buf = vec![];
        rom.read_to_end(&mut buf).unwrap();

        let mut app = App {
            gameboy: gb_core::gameboy::Gameboy::new(buf).unwrap(),
        };
        app.gameboy.reset();

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
        // static FRAMES: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        for _ in 0..gb_core::gameboy::ppu::monochrome::FRAME_T_CYCLES {
            self.gameboy.clock()
        }
        // println!(
        //     "frame {}",
        //     FRAMES.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        // );
        iced::Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let frame = self.gameboy.get_frame();
        let (tile_data, tilew, tileh) = self.gameboy.ppu.state.borrow().display_tile_data();
        iced::Row::new()
            // .push(iced::Text::new("Hello, world!"))
            .push(iced::Image::new(iced::image::Handle::from_pixels(
                160, 144, frame,
            )))
            .push(iced::Image::new(iced::image::Handle::from_pixels(
                tilew as u32,
                tileh as u32,
                u32_to_bgra(tile_data),
            )))
            .into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced_futures::time::every(std::time::Duration::from_millis(50)).map(|_| ())
    }

    fn background_color(&self) -> Color {
        Color::BLACK
    }
}

fn main() {
    let mut settings = Settings {
        flags: std::env::args().nth(1).expect("Expected 1 argument").into(),
        ..Default::default()
    };
    settings.window.min_size = Some((160, 144));
    App::run(settings).unwrap();
}

fn u32_to_bgra(x: Vec<u32>) -> Vec<u8> {
    x.iter().copied().flat_map(|p| p.to_le_bytes()).collect()
}
