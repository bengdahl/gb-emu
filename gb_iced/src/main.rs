use std::path::PathBuf;

use iced::{
    keyboard::{key::Named, Key},
    window, Application, Element, Length, Settings,
};

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
    type Theme = iced::theme::Theme;

    fn new(rom_path: PathBuf) -> (Self, iced::Command<Message>) {
        use std::io::Read;
        let mut rom = std::fs::File::open(rom_path).unwrap();
        let mut buf = vec![];
        rom.read_to_end(&mut buf).unwrap();

        let mut app = App {
            gameboy: gb_core::gameboy::Gameboy::new(buf).unwrap(),
            paused: false,
            log_instructions: false,
        };
        app.gameboy.reset();

        let cmd = iced::Command::none();
        (app, cmd)
    }

    fn title(&self) -> String {
        if !self.paused {
            "GameBoy".to_string()
        } else {
            "GameBoy - Paused".to_string()
        }
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Message> {
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

    fn view(&self) -> Element<'_, Self::Message> {
        let frame = self.gameboy.get_frame();
        let (tile_data, tilew, tileh) = self.gameboy.ppu.display_tile_data(2);
        iced::widget::Row::new()
            // .push(iced::Text::new("Hello, world!"))
            .push(
                iced::widget::Image::new(iced::widget::image::Handle::from_pixels(
                    160,
                    144,
                    u32_to_bgra(frame.iter().copied()),
                ))
                .width(Length::FillPortion(5))
                .height(Length::FillPortion(3)),
            )
            .push(
                iced::widget::Image::new(iced::widget::image::Handle::from_pixels(
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
        let key_press = iced::keyboard::on_key_press(|key, _| {
            if let Some(button) = keycode_to_button(&key) {
                Some(Message::Pressed(button))
            } else {
                match key {
                    Key::Character(c) if c == "p" => Some(Message::TogglePause),
                    Key::Character(c) if c == "d" => Some(Message::DebugCpu),
                    Key::Character(c) if c == "n" => Some(Message::StepInstruction),
                    Key::Character(c) if c == "l" => Some(Message::ToggleLog),
                    Key::Character(c) if c == "o" => Some(Message::DebugOam),
                    _ => None,
                }
            }
        });

        let key_release = iced::keyboard::on_key_release(|key, _| {
            let button = keycode_to_button(&key)?;
            Some(Message::Released(button))
        });

        iced::subscription::Subscription::batch([
            iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::TickFrame),
            key_press,
            key_release,
        ])
    }
}

fn main() {
    let mut settings = Settings {
        flags: std::env::args().nth(1).expect("Expected 1 argument").into(),
        window: window::Settings {
            size: iced::Size {
                width: 160.0 * 2.0,
                height: 144.0 * 2.0,
            },
            ..Default::default()
        },
        ..Default::default()
    };
    settings.window.min_size = Some(iced::Size {
        width: 160.0,
        height: 144.0,
    });
    App::run(settings).unwrap();
}

fn u32_to_bgra(x: impl Iterator<Item = u32>) -> Vec<u8> {
    x.flat_map(|p| p.to_le_bytes()).collect()
}

fn keycode_to_button(key: &Key) -> Option<gb_core::gameboy::joypad::Button> {
    match key {
        Key::Named(Named::ArrowUp) => Some(gb_core::gameboy::joypad::Button::Up),
        Key::Named(Named::ArrowLeft) => Some(gb_core::gameboy::joypad::Button::Left),
        Key::Named(Named::ArrowRight) => Some(gb_core::gameboy::joypad::Button::Right),
        Key::Named(Named::ArrowDown) => Some(gb_core::gameboy::joypad::Button::Down),
        Key::Character(c) if c == "z" => Some(gb_core::gameboy::joypad::Button::B),
        Key::Character(c) if c == "x" => Some(gb_core::gameboy::joypad::Button::A),
        Key::Character(c) if c == "g" => Some(gb_core::gameboy::joypad::Button::Select),
        Key::Character(c) if c == "h" => Some(gb_core::gameboy::joypad::Button::Start),
        _ => None,
    }
}
