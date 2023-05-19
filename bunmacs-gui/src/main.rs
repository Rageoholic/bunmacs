mod graphics;

use font_kit::{family_name::FamilyName, properties::Properties, source::SystemSource};
use graphics::WgpuInfo;

use std::collections::HashMap;
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoopBuilder,
    window::WindowBuilder,
};

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

    let mut window_contexts = HashMap::new();

    window_contexts.insert(window_context.id(), window_context);

    event_loop.run(move |event, _target, control_flow| match event {
        Event::WindowEvent {
            window_id,
            ref event,
        } => {
            if let Some(context) = window_contexts.get_mut(&window_id) {
                match event {
                    WindowEvent::CloseRequested => {
                        //TODO: confirm if user wants to close? Unsaved files?
                        window_contexts.remove(&window_id);
                        if window_contexts.len() == 0 {
                            control_flow.set_exit();
                        }
                    }

                    WindowEvent::Resized(new_size) => context.resize(*new_size),
                    _ => (),
                }
            } else {
                log::error!("Window context not found for window ID {:?}", window_id);
            }
        }

        Event::RedrawRequested(window_id) => {
            if let Some(context) = window_contexts.get_mut(&window_id) {
                context.redraw().expect("WGPU Surface Error");
            } else {
                log::error!("Invalid window ID passed to redraw.");
            }
        }

        _ => {}
    });
}
