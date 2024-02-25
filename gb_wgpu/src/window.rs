use std::sync::Arc;

use gb_core::gameboy::{joypad::Button, ppu::frame::Frame};
use smol::channel::Sender;
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[derive(Debug)]
pub enum ViewEvent {
    GameboyFrame { frame: Box<Frame> },
}

#[derive(Debug)]
pub enum InputEvent {
    ButtonPressed(gb_core::gameboy::joypad::Button),
    ButtonReleased(gb_core::gameboy::joypad::Button),
}

pub struct ViewSetup {
    event_loop: EventLoop<ViewEvent>,
    window: Arc<Window>,
    event_loop_proxy: EventLoopProxy<ViewEvent>,
    input_send: Sender<InputEvent>,
}

impl ViewSetup {
    pub fn new(input_send: Sender<InputEvent>) -> Self {
        let event_loop = winit::event_loop::EventLoopBuilder::with_user_event()
            .build()
            .unwrap();
        let window = Arc::new(
            winit::window::WindowBuilder::new()
                .build(&event_loop)
                .expect("Could not create window"),
        );
        let event_loop_proxy = event_loop.create_proxy();

        Self {
            event_loop,
            window,
            event_loop_proxy,
            input_send,
        }
    }

    pub fn event_loop_proxy(&self) -> EventLoopProxy<ViewEvent> {
        self.event_loop_proxy.clone()
    }

    /// Permanently blocks the current thread.
    pub fn run(self) {
        let surface = pixels::SurfaceTexture::new(
            self.window.inner_size().width,
            self.window.inner_size().height,
            self.window.as_ref(),
        );
        let mut pixels_ctx = pixels::PixelsBuilder::new(160, 144, surface)
            .render_texture_format(pixels::wgpu::TextureFormat::Bgra8UnormSrgb)
            .build()
            .unwrap();

        self.event_loop
            .run(move |event, elwt| match event {
                Event::WindowEvent {
                    event,
                    window_id: _window_id,
                } => match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(size) => {
                        pixels_ctx.resize_surface(size.width, size.height).unwrap();
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(key),
                                state,
                                ..
                            },
                        ..
                    } => match (state, key) {
                        (ElementState::Pressed, KeyCode::KeyP) => {
                            println!("Ping!");
                        }
                        (ElementState::Pressed, KeyCode::KeyB) => {
                            pixels_ctx
                                .frame_mut()
                                .chunks_mut(4)
                                .for_each(|pix| pix.copy_from_slice(&[0xFF, 0x00, 0x00, 0xFF]));
                        }
                        (state, key) if keycode_to_joypad(key).is_some() => smol::block_on(async {
                            let button = keycode_to_joypad(key).unwrap();
                            self.input_send
                                .send(match state {
                                    ElementState::Pressed => InputEvent::ButtonPressed(button),
                                    ElementState::Released => InputEvent::ButtonReleased(button),
                                })
                                .await
                                .unwrap()
                        }),
                        _ => {}
                    },
                    WindowEvent::RedrawRequested => {
                        pixels_ctx.render().unwrap();
                        elwt.set_control_flow(ControlFlow::Wait);
                    }
                    _ => {}
                },

                Event::UserEvent(event) => match event {
                    ViewEvent::GameboyFrame { frame } => {
                        let framebuffer = pixels_ctx.frame_mut();
                        let fb_pitch = 160 * 4;

                        for y in 0..144 {
                            for x in 0..160 {
                                let pix = frame[(x, y)];
                                let [r, g, b, a] = pix.to_le_bytes();

                                let fb_offset = y * fb_pitch + x * 4;
                                framebuffer[fb_offset] = r;
                                framebuffer[fb_offset + 1] = g;
                                framebuffer[fb_offset + 2] = b;
                                framebuffer[fb_offset + 3] = a;
                            }
                        }

                        self.window.request_redraw();
                    }
                },
                _ => {}
            })
            .unwrap()
    }
}

fn keycode_to_joypad(key: KeyCode) -> Option<Button> {
    match key {
        KeyCode::KeyZ => Some(Button::A),
        KeyCode::KeyX => Some(Button::B),
        KeyCode::KeyG => Some(Button::Select),
        KeyCode::KeyH => Some(Button::Start),
        KeyCode::ArrowUp => Some(Button::Up),
        KeyCode::ArrowDown => Some(Button::Down),
        KeyCode::ArrowLeft => Some(Button::Left),
        KeyCode::ArrowRight => Some(Button::Right),
        _ => None,
    }
}
