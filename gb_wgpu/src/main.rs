#![feature(try_blocks)]

use std::sync::Arc;

use gb_core::gameboy::Gameboy;
use smol::channel::Receiver;

use smol::lock::Mutex;
use smol::stream::StreamExt;
use window::{InputEvent, ViewEvent};

mod window;

fn main() {
    let rom_path = std::env::args().nth(1).expect("Expected path to ROM");
    let rom_data = std::fs::read(rom_path).unwrap();
    let gameboy = Gameboy::new(rom_data).unwrap();

    let (input_send, input_recv) = smol::channel::bounded(8);

    let view = window::ViewSetup::new(input_send);
    let event_loop_proxy = view.event_loop_proxy();

    std::thread::spawn(move || game_thread(gameboy, input_recv, event_loop_proxy));

    // ViewSetup is not Send or Sync, so it has to run on the thread it was made on.
    view.run()
}

fn game_thread(
    mut gameboy: gb_core::gameboy::Gameboy,
    input_recv: Receiver<window::InputEvent>,
    event_loop_proxy: winit::event_loop::EventLoopProxy<window::ViewEvent>,
) {
    let exec = smol::Executor::new();

    gameboy.reset();

    let gameboy = Arc::new(Mutex::new(gameboy));

    // Input handler
    exec.spawn({
        let gameboy = gameboy.clone();
        async move {
            loop {
                let input = input_recv.recv().await.unwrap();
                match input {
                    InputEvent::ButtonPressed(button) => gameboy.lock().await.joypad.press(button),
                    InputEvent::ButtonReleased(button) => {
                        gameboy.lock().await.joypad.release(button)
                    }
                }
            }
        }
    })
    .detach();

    // Gameboy runner loop
    smol::block_on(exec.run(async {
        let mut frame_timer = smol::Timer::interval(std::time::Duration::from_millis(16));
        while let Some(_) = frame_timer.next().await {
            let mut gameboy = gameboy.lock().await;
            for _ in 0..gb_core::gameboy::ppu::consts::FRAME_T_CYCLES / 4 {
                gameboy.clock();
            }

            let frame = gameboy.get_frame();

            if let Err(_) = event_loop_proxy.send_event(ViewEvent::GameboyFrame { frame }) {
                break;
            }
        }
    }));
}
