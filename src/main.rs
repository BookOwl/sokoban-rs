extern crate sdl2;
extern crate fps_clock;

use sdl2::video::Window;
use sdl2::render::Canvas;
use sdl2::EventPump;
use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

use fps_clock::FpsClock;


fn init_sdl(app_name: &str, width: u32, height: u32) -> Result<(Canvas<Window>, EventPump), String> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem.window(app_name, width, height)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| format!("{}", e))?;

    let canvas = window.into_canvas().build().map_err(|e| format!("{}", e))?;
    let event_pump = sdl_context.event_pump()?;
    Ok((canvas, event_pump))
}

fn main() {
    let (mut canvas, mut event_pump) = init_sdl("Sokoban", 800, 600).unwrap();
    let mut clock = FpsClock::new(60);
    'main: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => {
                    break 'main
                },
                _ => {}
            }
        }
        canvas.set_draw_color(Color::RGB(29, 167, 226));
        canvas.clear();
        canvas.present();
        clock.tick();
    }
}
