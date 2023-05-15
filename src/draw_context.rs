use std::{iter, sync::Arc};

use tokio::runtime::Runtime;
use wgpu::{
    Backends, Color, CommandEncoderDescriptor, Device, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, LoadOp, Operations, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RequestAdapterOptions, Surface, SurfaceConfiguration, SurfaceError, TextureUsages,
};
use winit::{
    dpi::PhysicalSize,
    window::{Window, WindowId},
};

//TODO: Genericize over backend?

pub(crate) struct WgpuInfo {
    _shared_context: Arc<SharedWgpuContext>,
}

struct SharedWgpuContext {
    _instance: Instance,
    //currently unknown if we need this?
    //_adapter: Adapter,
    device: Device,
    _queue: Queue,
}

impl WgpuInfo {
    pub(crate) fn new(initial_window: Window, rt: &Runtime) -> (Self, WindowContext) {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let win_size = initial_window.inner_size();

        //#SAFETY

        //This unsafe is necessary because initial_window must live as long as
        //the surface or longer. Basically there's a lifetime here that's not
        //enforced by the type system
        let surface = unsafe { instance.create_surface(&initial_window).unwrap() };

        let adapter = rt
            .block_on(instance.request_adapter(&RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            }))
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .filter(|f| f.is_srgb())
            .next()
            .unwrap_or(surface_caps.formats[0]);

        let (device, queue) = rt
            .block_on(adapter.request_device(
                &DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                },
                None,
            ))
            .unwrap();

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: win_size.width,
            height: win_size.height,
            //Present_Mode::Fifo, guaranteed to exist and good enough for our
            //purposes as an editor
            present_mode: surface_caps.present_modes[0],
            //seems we can just paste whatever here so long as it's supported?
            //TODO: look into later
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };
        if win_size.width != 0 && win_size.height != 0 {
            surface.configure(&device, &config);
        }

        let inner = Arc::new(SharedWgpuContext {
            _instance: instance,
            //_adapter: adapter,
            device,
            _queue: queue,
        });
        (
            WgpuInfo {
                _shared_context: inner.clone(),
            },
            WindowContext {
                surface,
                win: initial_window,
                wgpu_info: inner,
                inner_size: win_size,
                surface_config: config,
            },
        )
    }
}

pub(crate) struct WindowContext {
    surface: Surface,
    win: Window,
    wgpu_info: Arc<SharedWgpuContext>,
    surface_config: SurfaceConfiguration,
    inner_size: PhysicalSize<u32>,
}

impl WindowContext {
    pub fn id(&self) -> WindowId {
        self.win.id()
    }

    pub fn set_visible(&self, b: bool) {
        self.win.set_visible(b)
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.inner_size = new_size;

        if new_size.width != 0 && new_size.height != 0 {
            self.surface
                .configure(&self.wgpu_info.device, &self.surface_config);
        }
    }

    pub fn redraw(&self) -> Result<(), SurfaceError> {
        if self.inner_size.width != 0 && self.inner_size.height != 0 {
            let output = self.surface.get_current_texture()?;
            let view = output.texture.create_view(&Default::default());
            let mut encoder =
                self.wgpu_info
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });
            let render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1f64,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            drop(render_pass);

            self.wgpu_info._queue.submit(iter::once(encoder.finish()));
            output.present();
        }
        Ok(())
    }
}
