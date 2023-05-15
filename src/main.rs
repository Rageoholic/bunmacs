mod draw_context;

use std::collections::HashMap;

use draw_context::WgpuInfo;
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
        .with_resizable(false)
        .with_visible(false)
        .build(&ev)
        .unwrap();
    let (_wgpu_handle, win) = WgpuInfo::new(win, &tokio_rt);
    let mut wins = HashMap::new();
    wins.insert(win.id(), win);
    ev.run(move |event, _target, control_flow| match event {
        Event::WindowEvent {
            window_id: win_id,
            event: WindowEvent::CloseRequested,
        } => {
            if let Some(_) = wins.get(&win_id) {
                {
                    //TODO: confirm if user wants to close? Unsaved files?
                    wins.remove(&win_id);
                    if wins.len() == 0 {
                        control_flow.set_exit();
                    }
                }
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
