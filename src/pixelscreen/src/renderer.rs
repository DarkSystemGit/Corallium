//! wgpu plumbing: surface/device/pipeline setup, texture upload, and the
//! per-frame draw call. Scaling always fits the buffer's height to the
//! window's height, pillarboxing the sides -- computed entirely on the
//! GPU in `shader.wgsl`'s vertex stage. The pixel buffer is uploaded
//! completely unmodified as a raw `R32Uint` texture and unpacked by the
//! fragment shader according to [`PixelFormat`] -- this module never
//! byte-shuffles your buffer on the CPU. See [`crate::scale`] for a
//! CPU-side reference implementation of the scaling math (handy for
//! hit-testing).

use crate::scale::ScaleMode;
use std::sync::Arc;
use wgpu::util::DeviceExt;
use winit::window::Window as WinitWindow;

/// RGBA color used to fill the pillarbox bars on the sides.
#[derive(Debug, Clone, Copy)]
pub struct BarColor {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Default for BarColor {
    fn default() -> Self {
        // Opaque black.
        BarColor {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }
}

/// How to interpret each `u32` in the buffer passed to [`crate::Window::render`].
/// Named after how the value reads as a hex literal, so you can match it
/// to your own buffer's packing at a glance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    /// `0xAABBGGRR` -- alpha in the highest byte, red in the lowest:
    /// `pixel = r | g << 8 | b << 16 | a << 24`. pixelscreen's original
    /// (and still default) format.
    Aabbggrr,
    /// `0xRRGGBBAA` -- red in the highest byte, alpha in the lowest:
    /// `pixel = r << 24 | g << 16 | b << 8 | a`.
    Rrggbbaa,
}

impl Default for PixelFormat {
    fn default() -> Self {
        PixelFormat::Aabbggrr
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RendererError {
    #[error("failed to find a compatible GPU adapter")]
    NoAdapter,
    #[error("failed to request GPU device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("failed to create surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),
    #[error("buffer size ({buf_w}x{buf_h}) does not match {len} pixels passed to render()")]
    BufferSizeMismatch { buf_w: u32, buf_h: u32, len: usize },
}

/// Mirrors `FrameUniform` in `shader.wgsl` exactly: two `vec2<f32>`
/// (8-byte aligned) followed by four `f32`s, for a 16-byte-aligned total
/// size of 32 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct FrameUniform {
    buf_size: [f32; 2],
    win_size: [f32; 2],
    scale_mode: f32,
    pixel_format: f32,
    _pad1: f32,
    _pad2: f32,
}

impl FrameUniform {
    fn new(
        buf_w: u32,
        buf_h: u32,
        win_w: u32,
        win_h: u32,
        scale_mode: ScaleMode,
        pixel_format: PixelFormat,
    ) -> Self {
        Self {
            buf_size: [buf_w as f32, buf_h as f32],
            win_size: [win_w as f32, win_h as f32],
            scale_mode: match scale_mode {
                ScaleMode::Integer => 0.0,
                ScaleMode::Fit => 1.0,
            },
            pixel_format: match pixel_format {
                PixelFormat::Aabbggrr => 0.0,
                PixelFormat::Rrggbbaa => 1.0,
            },
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}
#[derive(Debug)]
pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    frame_uniform_buffer: wgpu::Buffer,

    // Texture sized to the pixel buffer; recreated only if buffer size changes.
    texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
    tex_w: u32,
    tex_h: u32,

    pub bar_color: BarColor,
    pub scale_mode: ScaleMode,
    pub pixel_format: PixelFormat,
}

impl Renderer {
    pub fn new(window: Arc<WinitWindow>, buf_w: u32, buf_h: u32) -> Result<Self, RendererError> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window)?;

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .ok_or(RendererError::NoAdapter)?;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("pixelscreen device"),
                required_features: wgpu::Features::empty(),
                required_limits:
                    wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            },
            None,
        ))?;

        let surface_caps = surface.get_capabilities(&adapter);
        // Deliberately NOT srgb: the fragment shader unpacks raw 0-255
        // byte values straight to [0,1] with no gamma decode (matching
        // minifb-style "buffer bytes are exactly what gets displayed"
        // behavior). Writing those un-decoded values to an srgb target
        // would silently re-encode them, washing out midtones.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo, // vsync, avoids tearing
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pixelscreen bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    // Raw uint texture -- no sampler, no format-driven
                    // reinterpretation. The buffer's bytes land in the
                    // texture completely unchanged; the fragment shader
                    // unpacks them itself per `pixel_format`.
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let scale_mode = ScaleMode::default();
        let pixel_format = PixelFormat::default();
        let frame_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("pixelscreen frame uniform buffer"),
            contents: bytemuck::bytes_of(&FrameUniform::new(
                buf_w,
                buf_h,
                surface_config.width,
                surface_config.height,
                scale_mode,
                pixel_format,
            )),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pixelscreen shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pixelscreen pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pixelscreen pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let (texture, bind_group) = create_texture_and_bind_group(
            &device,
            &bind_group_layout,
            &frame_uniform_buffer,
            buf_w,
            buf_h,
        );

        Ok(Self {
            surface,
            device,
            queue,
            surface_config,
            pipeline,
            bind_group_layout,
            frame_uniform_buffer,
            texture,
            bind_group,
            tex_w: buf_w,
            tex_h: buf_h,
            bar_color: BarColor::default(),
            scale_mode,
            pixel_format,
        })
    }

    /// Call when the window is resized.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn window_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    /// Uploads `pixels` completely unmodified (RGBA, row-major,
    /// `buf_w * buf_h` u32s, byte order per `self.pixel_format`) and draws
    /// it into the current surface texture. The GPU scales it to fit the
    /// window's height exactly and pillarboxes the sides, per
    /// `self.scale_mode` -- see `shader.wgsl`.
    pub fn render(&mut self, pixels: &[u32], buf_w: u32, buf_h: u32) -> Result<(), RendererError> {
        if pixels.len() != (buf_w as usize) * (buf_h as usize) {
            return Err(RendererError::BufferSizeMismatch {
                buf_w,
                buf_h,
                len: pixels.len(),
            });
        }

        if buf_w != self.tex_w || buf_h != self.tex_h {
            let (texture, bind_group) = create_texture_and_bind_group(
                &self.device,
                &self.bind_group_layout,
                &self.frame_uniform_buffer,
                buf_w,
                buf_h,
            );
            self.texture = texture;
            self.bind_group = bind_group;
            self.tex_w = buf_w;
            self.tex_h = buf_h;
        }

        // Upload pixel data to the texture, byte-for-byte unchanged --
        // the fragment shader is what interprets these bytes.
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(pixels),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * buf_w),
                rows_per_image: Some(buf_h),
            },
            wgpu::Extent3d {
                width: buf_w,
                height: buf_h,
                depth_or_array_layers: 1,
            },
        );

        // Feed the GPU everything it needs to compute scale + pillarbox
        // placement and unpack pixels itself: buffer size, current window
        // size, scale mode, pixel format.
        let uniform = FrameUniform::new(
            buf_w,
            buf_h,
            self.surface_config.width,
            self.surface_config.height,
            self.scale_mode,
            self.pixel_format,
        );
        self.queue
            .write_buffer(&self.frame_uniform_buffer, 0, bytemuck::bytes_of(&uniform));

        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("pixelscreen encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("pixelscreen render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // Clearing to the bar color paints the pillarbox bars
                        // in one step -- the vertex shader positions the quad
                        // to cover only the scaled image area, so anything outside
                        // it simply never gets touched by the draw call below.
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: self.bar_color.r,
                            g: self.bar_color.g,
                            b: self.bar_color.b,
                            a: self.bar_color.a,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.draw(0..6, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        Ok(())
    }
}

fn create_texture_and_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    frame_uniform_buffer: &wgpu::Buffer,
    buf_w: u32,
    buf_h: u32,
) -> (wgpu::Texture, wgpu::BindGroup) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("pixelscreen buffer texture"),
        size: wgpu::Extent3d {
            width: buf_w.max(1),
            height: buf_h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // One u32 per texel, uploaded raw -- see module docs.
        format: wgpu::TextureFormat::R32Uint,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });

    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("pixelscreen bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: frame_uniform_buffer.as_entire_binding(),
            },
        ],
    });

    (texture, bind_group)
}
