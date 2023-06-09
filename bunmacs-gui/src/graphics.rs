use std::{cell::RefCell, fmt::Debug, iter, mem::size_of_val, num::NonZeroU64, sync::Arc};

use tokio::runtime::Runtime;
use wgpu::{
    util::StagingBelt, Backends, BlendState, Buffer, BufferDescriptor, BufferUsages, Color,
    ColorTargetState, ColorWrites, CommandEncoderDescriptor, Device, DeviceDescriptor, Features,
    FragmentState, Instance, InstanceDescriptor, LoadOp, MultisampleState, Operations,
    PipelineLayoutDescriptor, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RenderPipeline, RenderPipelineDescriptor, RequestAdapterOptions, ShaderModuleDescriptor,
    Surface, SurfaceConfiguration, SurfaceError, TextureUsages, VertexAttribute,
    VertexBufferLayout, VertexFormat, VertexState,
};
use wgpu_glyph::{ab_glyph::FontArc, GlyphBrush, GlyphBrushBuilder, Section, Text};

use winit::{
    dpi::PhysicalSize,
    window::{Window, WindowId},
};

//TODO: Genericize over backend?
#[derive(Debug)]
pub(crate) struct WgpuInfo {
    _shared_context: Arc<SharedWgpuContext>,
}

struct SharedWgpuContext {
    _instance: Instance,
    //currently unknown if we need this?
    //_adapter: Adapter,
    device: Device,
    queue: Queue,
    render_pipeline: RenderPipeline,
    vertex_buffer: Buffer,
    glyph_brush: RefCell<GlyphBrush<()>>,
    staging_belt: RefCell<StagingBelt>,
}

impl Debug for SharedWgpuContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedWgpuContext")
            .field("_instance", &self._instance)
            .field("device", &self.device)
            .field("queue", &self.queue)
            .field("render_pipeline", &self.render_pipeline)
            .field("vertex_buffer", &self.vertex_buffer)
            .field("glyph_brush", &self.glyph_brush.borrow())
            .finish()
    }
}

#[derive(Debug)]
pub(crate) struct WindowContext {
    surface: Surface,
    wgpu_info: Arc<SharedWgpuContext>,
    surface_config: SurfaceConfiguration,
    win: Window,
    inner_size: PhysicalSize<u32>,
}

// lib.rs
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];

impl WgpuInfo {
    pub(crate) fn new(
        win: Window,
        rt: &Runtime,
        font: &font_kit::font::Font,
    ) -> (Self, WindowContext) {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let inner_size = win.inner_size();

        //#SAFETY

        //This unsafe is necessary because initial_window must live as long as
        //the surface or longer. Basically there's a lifetime here that's not
        //enforced by the type system
        let surface = unsafe { instance.create_surface(&win).unwrap() };

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

        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("render pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(ColorTargetState {
                    format: surface_format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: inner_size.width,
            height: inner_size.height,
            //Present_Mode::Fifo, guaranteed to exist and good enough for our
            //purposes as an editor
            present_mode: surface_caps.present_modes[0],
            //seems we can just paste whatever here so long as it's supported?
            //TODO: look into later
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        let vertices_size: u64 = size_of_val(VERTICES) as u64;

        if inner_size.width != 0 && inner_size.height != 0 {
            surface.configure(&device, &surface_config);
        }

        let vertex_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("vertex buffer"),
            size: vertices_size,
            usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let font_bytes = font.copy_font_data().unwrap();

        let glyph_font = FontArc::try_from_vec((*font_bytes).clone()).unwrap();
        let glyph_brush =
            GlyphBrushBuilder::using_font(glyph_font).build(&device, surface_config.format);

        let staging_belt = StagingBelt::new(64);

        let wgpu_info = Arc::new(SharedWgpuContext {
            _instance: instance,
            //_adapter: adapter,
            device,
            queue,
            render_pipeline,
            vertex_buffer,
            staging_belt: RefCell::new(staging_belt),
            glyph_brush: RefCell::new(glyph_brush),
        });

        (
            WgpuInfo {
                _shared_context: wgpu_info.clone(),
            },
            WindowContext {
                surface,
                win,
                wgpu_info,
                inner_size,
                surface_config,
            },
        )
    }
}

impl WindowContext {
    pub fn id(&self) -> WindowId {
        self.win.id()
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

    pub fn redraw(&mut self) -> Result<(), SurfaceError> {
        if self.inner_size.width != 0 && self.inner_size.height != 0 {
            let output = self.surface.get_current_texture()?;
            let view = output.texture.create_view(&Default::default());
            let mut encoder =
                self.wgpu_info
                    .device
                    .create_command_encoder(&CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });

            let mut staging_belt = self.wgpu_info.staging_belt.borrow_mut();

            staging_belt
                .write_buffer(
                    &mut encoder,
                    &self.wgpu_info.vertex_buffer,
                    0,
                    NonZeroU64::new(size_of_val(VERTICES) as u64).unwrap(),
                    &self.wgpu_info.device,
                )
                .copy_from_slice(bytemuck::cast_slice(&VERTICES[..]));

            //Do our render pass here
            {
                let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
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
                render_pass.set_pipeline(&self.wgpu_info.render_pipeline);
                render_pass.set_vertex_buffer(0, self.wgpu_info.vertex_buffer.slice(..));
                render_pass.draw(0..3, 0..1);
            }

            {
                let glyph_brush = &mut self.wgpu_info.glyph_brush.borrow_mut();
                glyph_brush.queue(Section {
                    screen_position: (50.0, 90.0),
                    bounds: (
                        self.surface_config.width as f32,
                        self.surface_config.height as f32,
                    ),
                    text: vec![Text::new("Hello world")
                        .with_color([1.0, 1.0, 1.0, 1.0])
                        .with_scale(40.0)],
                    ..Default::default()
                });

                glyph_brush.queue(Section {
                    screen_position: (180.0, 360.0),
                    bounds: (
                        self.surface_config.width as f32,
                        self.surface_config.height as f32,
                    ),
                    text: vec![Text::new("Hello world")
                        .with_color([1.0, 0.0, 0.0, 1.0])
                        .with_scale(13.0)],
                    ..Default::default()
                });

                glyph_brush
                    .draw_queued(
                        &self.wgpu_info.device,
                        &mut staging_belt,
                        &mut encoder,
                        &view,
                        self.surface_config.width,
                        self.surface_config.height,
                    )
                    .unwrap();
            }
            staging_belt.finish();
            self.wgpu_info.queue.submit(iter::once(encoder.finish()));
            output.present();
        }
        Ok(())
    }
}

impl Vertex {
    const POSITION_OFFSET: usize = 0;
    const COLOR_OFFSET: usize = 12;
    fn desc<'a>() -> VertexBufferLayout<'a> {
        assert!(memoffset::offset_of!(Vertex, position) == Self::POSITION_OFFSET);
        assert!(memoffset::offset_of!(Vertex, color) == Self::COLOR_OFFSET);
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    format: VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                VertexAttribute {
                    format: VertexFormat::Float32x4,
                    offset: std::mem::size_of::<[f32; 3]>() as u64,
                    shader_location: 1,
                },
            ],
        }
    }
}
