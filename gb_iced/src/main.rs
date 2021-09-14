use std::path::PathBuf;

use iced::{keyboard::KeyCode, window, Application, Color, Element, Length, Settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Message {
    Pressed(gb_core::gameboy::joypad::Button),
    Released(gb_core::gameboy::joypad::Button),
    TickFrame,
    TogglePause,
    DebugCpu,
    StepInstruction,
    ToggleLog,
    DebugOam,
}

struct App {
    gameboy: gb_core::gameboy::Gameboy,
    paused: bool,
    log_instructions: bool,
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Flags = PathBuf;
    type Message = Message;

    fn new(rom_path: PathBuf) -> (Self, iced::Command<Message>) {
        use std::io::Read;
        let mut rom = std::fs::File::open(rom_path).unwrap();
        let mut buf = vec![];
        rom.read_to_end(&mut buf).unwrap();

        let mut app = App {
            gameboy: gb_core::gameboy::Gameboy::new(buf).unwrap(),
            paused: true,
            log_instructions: false,
        };
        app.gameboy.reset();

        let cmd = iced::Command::none();
        (app, cmd)
    }

    fn title(&self) -> String {
        if !self.paused {
            format!("GameBoy")
        } else {
            format!("GameBoy - Paused")
        }
    }

    fn update(
        &mut self,
        message: Self::Message,
        _clip: &mut iced::Clipboard,
    ) -> iced::Command<Message> {
        match message {
            Message::TickFrame => {
                if !self.paused {
                    for _ in 0..gb_core::gameboy::ppu::consts::FRAME_T_CYCLES / 4 {
                        let debug_info = self.gameboy.clock();
                        if self.log_instructions && debug_info.is_fetch_cycle {
                            println!("{:?}", self.gameboy.cpu);
                        }
                    }
                }
                iced::Command::none()
            }

            Message::Pressed(button) => {
                self.gameboy.joypad.press(button);
                iced::Command::none()
            }
            Message::Released(button) => {
                self.gameboy.joypad.release(button);
                iced::Command::none()
            }

            Message::TogglePause => {
                self.paused = !self.paused;
                iced::Command::none()
            }

            Message::DebugCpu => {
                println!("{:?}", self.gameboy.cpu);
                iced::Command::none()
            }
            Message::StepInstruction => {
                self.gameboy.step_instruction();
                println!("{:?}", self.gameboy.cpu);
                iced::Command::none()
            }
            Message::DebugOam => {
                println!(
                    "{:?}",
                    (0..40)
                        .filter_map(|i| {
                            let entry = self.gameboy.ppu.oam(i);
                            if entry != Default::default() {
                                Some(entry)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                );
                iced::Command::none()
            }

            Message::ToggleLog => {
                self.log_instructions = !self.log_instructions;
                if self.log_instructions {
                    println!("Logging ON");
                } else {
                    println!("Logging OFF");
                }
                iced::Command::none()
            }
        }
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let frame = self.gameboy.get_frame();
        let (tile_data, tilew, tileh) = self.gameboy.ppu.display_tile_data(2);
        iced::Row::new()
            // .push(iced::Text::new("Hello, world!"))
            .push(
                iced::Image::new(iced::image::Handle::from_pixels(
                    160 as u32,
                    144 as u32,
                    u32_to_bgra(frame.iter().copied()),
                ))
                .width(Length::FillPortion(5))
                .height(Length::FillPortion(3)),
            )
            .push(
                iced::Image::new(iced::image::Handle::from_pixels(
                    tilew as u32,
                    tileh as u32,
                    u32_to_bgra(tile_data.iter().copied()),
                ))
                .width(Length::FillPortion(4))
                .height(Length::FillPortion(4)),
            )
            .into()
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        iced_futures::subscription::Subscription::batch([
            iced_futures::time::every(std::time::Duration::from_millis(16))
                .map(|_| Message::TickFrame),
            iced_native::subscription::events_with(|event, _status| match event {
                iced_native::Event::Keyboard(e) => match e {
                    iced::keyboard::Event::KeyPressed { key_code, .. } => {
                        keycode_to_button(key_code)
                            .map(Message::Pressed)
                            .or_else(|| match key_code {
                                KeyCode::P => Some(Message::TogglePause),
                                KeyCode::D => Some(Message::DebugCpu),
                                KeyCode::N => Some(Message::StepInstruction),
                                KeyCode::L => Some(Message::ToggleLog),
                                KeyCode::O => Some(Message::DebugOam),
                                _ => None,
                            })
                    }
                    iced::keyboard::Event::KeyReleased { key_code, .. } => {
                        keycode_to_button(key_code).map(Message::Released)
                    }
                    _ => None,
                },
                _ => None,
            }),
        ])
    }

    fn background_color(&self) -> Color {
        Color::BLACK
    }
}

fn main() {
    let mut settings = Settings {
        flags: std::env::args().nth(1).expect("Expected 1 argument").into(),
        window: window::Settings {
            size: (160 * 2, 144 * 2),
            ..Default::default()
        },
        ..Default::default()
    };
    settings.window.min_size = Some((160, 144));
    App::run(settings).unwrap();
}

fn u32_to_bgra(x: impl Iterator<Item = u32>) -> Vec<u8> {
    x.flat_map(|p| p.to_le_bytes()).collect()
}

fn keycode_to_button(key_code: KeyCode) -> Option<gb_core::gameboy::joypad::Button> {
    match key_code {
        iced::keyboard::KeyCode::Up => Some(gb_core::gameboy::joypad::Button::Up),
        iced::keyboard::KeyCode::Left => Some(gb_core::gameboy::joypad::Button::Left),
        iced::keyboard::KeyCode::Right => Some(gb_core::gameboy::joypad::Button::Right),
        iced::keyboard::KeyCode::Down => Some(gb_core::gameboy::joypad::Button::Down),
        iced::keyboard::KeyCode::Z => Some(gb_core::gameboy::joypad::Button::B),
        iced::keyboard::KeyCode::X => Some(gb_core::gameboy::joypad::Button::A),
        iced::keyboard::KeyCode::G => Some(gb_core::gameboy::joypad::Button::Select),
        iced::keyboard::KeyCode::H => Some(gb_core::gameboy::joypad::Button::Start),
        _ => None,
    }
}
