use std::sync::Arc;

use gb_core::gameboy::joypad::Button;
use smol::channel::Sender;
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop, EventLoopProxy},
    window::Window,
};

#[derive(Debug)]
pub enum ViewEvent {
    GameboyFrame {
        pixels: Vec<u32>,
        width: usize,
        height: usize,
    },
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
        let event_loop = winit::event_loop::EventLoop::with_user_event();
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
    pub fn run(self) -> ! {
        let surface = pixels::SurfaceTexture::new(
            self.window.inner_size().width,
            self.window.inner_size().height,
            self.window.as_ref(),
        );
        let mut pixels_ctx = pixels::PixelsBuilder::new(160, 144, surface)
            .render_texture_format(wgpu::TextureFormat::Bgra8UnormSrgb)
            .build()
            .unwrap();

        self.event_loop
            .run(move |event, _, control_flow| match event {
                Event::WindowEvent {
                    event,
                    window_id: _window_id,
                } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        pixels_ctx.resize_surface(size.width, size.height);
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    } => match (state, key) {
                        (ElementState::Pressed, VirtualKeyCode::P) => {
                            println!("Ping!");
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
                    _ => {}
                },

                Event::UserEvent(event) => match event {
                    ViewEvent::GameboyFrame {
                        pixels,
                        width,
                        height,
                    } => {
                        let framebuffer = pixels_ctx.get_frame();
                        let fb_pitch = 160 * 4;

                        for row in 0..height {
                            for col in 0..width {
                                let pix = pixels[row * width + col];
                                let [r, g, b, a] = pix.to_le_bytes();

                                let fb_offset = row * fb_pitch + col * 4;
                                framebuffer[fb_offset + 0] = b;
                                framebuffer[fb_offset + 1] = g;
                                framebuffer[fb_offset + 2] = r;
                                framebuffer[fb_offset + 3] = a;
                            }
                        }

                        self.window.request_redraw();
                    }
                },

                Event::RedrawRequested(_) => {
                    pixels_ctx.render().unwrap();
                    *control_flow = ControlFlow::Wait;
                }
                _ => {}
            })
    }
}

fn keycode_to_joypad(key: VirtualKeyCode) -> Option<Button> {
    match key {
        VirtualKeyCode::Z => Some(Button::A),
        VirtualKeyCode::X => Some(Button::B),
        VirtualKeyCode::G => Some(Button::Select),
        VirtualKeyCode::H => Some(Button::Start),
        VirtualKeyCode::Up => Some(Button::Up),
        VirtualKeyCode::Down => Some(Button::Down),
        VirtualKeyCode::Left => Some(Button::Left),
        VirtualKeyCode::Right => Some(Button::Right),
        _ => None,
    }
}
