mod graphics;

use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
use graphics::{WgpuInfo, WindowContext};

use std::collections::{HashMap, HashSet};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};

enum Win {
    WindowContext(WindowContext),
    Tombstone,
}
fn main() {
    env_logger::init();

    let async_runtime = tokio::runtime::Builder::new_multi_thread().build().unwrap();

    let event_loop = EventLoopBuilder::new().build();

    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize {
            width: 1280,
            height: 720,
        })
        .with_title("Bunmacs!")
        .build(&event_loop)
        .unwrap();
    let font = SystemSource::new()
        .select_best_match(&[FamilyName::Monospace], &Properties::new())
        .unwrap()
        .load()
        .unwrap();

    let (_, window_context) = WgpuInfo::new(window, &async_runtime, &font);

    let mut window_set = HashSet::new();
    window_set.insert(window_context.id());

    let mut window_contexts = HashMap::new();

    window_contexts.insert(window_context.id(), Win::WindowContext(window_context));

    event_loop.run(move |event, _target, control_flow| match event {
        Event::WindowEvent {
            window_id,
            ref event,
        } => {
            if let Some(win) = window_contexts.get_mut(&window_id) {
                match event {
                    WindowEvent::CloseRequested => {
                        //TODO: confirm if user wants to close? Unsaved files?

                        *win = Win::Tombstone;
                    }

                    WindowEvent::Resized(new_size) => {
                        if let Win::WindowContext(context) = win {
                            context.resize(*new_size)
                        }
                    }
                    WindowEvent::Destroyed => {
                        window_contexts.remove(&window_id);
                        if window_contexts.len() == 0 {
                            *control_flow = ControlFlow::Exit;
                        }
                    }
                    _ => (),
                }
            } else {
                log::error!("Window context not found for window ID {:?}", window_id);
            }
        }

        Event::RedrawRequested(window_id) => {
            if let Some(Win::WindowContext(context)) = window_contexts.get_mut(&window_id) {
                context.redraw().expect("WGPU Surface Error");
            } else {
                log::error!("Invalid window ID passed to redraw.");
            }
        }

        _ => {}
    });
}
