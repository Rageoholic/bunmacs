mod graphics;

use graphics::WgpuInfo;
use std::collections::HashMap;
use winit::{
    event::{Event, StartCause, WindowEvent},
    event_loop::EventLoopBuilder,
    window::WindowBuilder,
};

fn main() {
    env_logger::init();

    let tokio_rt = tokio::runtime::Builder::new_multi_thread().build().unwrap();

    let ev = EventLoopBuilder::new().build();

    let win = WindowBuilder::new()
        .with_title("Bunmacs!")
        .with_visible(false)
        .build(&ev)
        .unwrap();

    let (_wgpu_handle, win) = WgpuInfo::new(win, &tokio_rt);

    let mut wins = HashMap::new();

    wins.insert(win.id(), win);

    ev.run(move |event, _target, control_flow| match event {
        Event::WindowEvent {
            window_id,
            ref event,
        } => {
            if let Some(win) = wins.get_mut(&window_id) {
                match event {
                    WindowEvent::CloseRequested => {
                        //TODO: confirm if user wants to close? Unsaved files?
                        wins.remove(&window_id);
                        if wins.len() == 0 {
                            control_flow.set_exit();
                        }
                    }

                    WindowEvent::Resized(new_size) => win.resize(*new_size),
                    _ => (),
                }
            }
        }

        Event::RedrawRequested(window_id) => {
            if let Some(win) = wins.get_mut(&window_id) {
                win.redraw().expect("WGPU Surface Error");
            }
        }

        Event::NewEvents(StartCause::Init) => {
            for (_, win) in &wins {
                win.set_visible(true);
            }
        }
        _ => {}
    })
}
