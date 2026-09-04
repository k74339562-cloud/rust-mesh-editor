use std::sync::Arc;
use glam::{Mat4, Vec3};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    platform::android::activity::AndroidApp,
    platform::android::EventLoopBuilderExtAndroid,
    window::{Window, WindowId},
};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    normal: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

// بيانات المكعب الافتراضي (24 نقطة للحصول على زوايا ونورمالز حادة)
const CUBE_VERTICES: &[Vertex] = &[
    // الوجه الأمامي (Z+)
    Vertex { position: [-1.0, -1.0,  1.0], normal: [ 0.0,  0.0,  1.0] },
    Vertex { position: [ 1.0, -1.0,  1.0], normal: [ 0.0,  0.0,  1.0] },
    Vertex { position: [ 1.0,  1.0,  1.0], normal: [ 0.0,  0.0,  1.0] },
    Vertex { position: [-1.0,  1.0,  1.0], normal: [ 0.0,  0.0,  1.0] },
    // الوجه الخلفي (Z-)
    Vertex { position: [ 1.0, -1.0, -1.0], normal: [ 0.0,  0.0, -1.0] },
    Vertex { position: [-1.0, -1.0, -1.0], normal: [ 0.0,  0.0, -1.0] },
    Vertex { position: [-1.0,  1.0, -1.0], normal: [ 0.0,  0.0, -1.0] },
    Vertex { position: [ 1.0,  1.0, -1.0], normal: [ 0.0,  0.0, -1.0] },
    // الوجه العلوي (Y+)
    Vertex { position: [-1.0,  1.0,  1.0], normal: [ 0.0,  1.0,  0.0] },
    Vertex { position: [ 1.0,  1.0,  1.0], normal: [ 0.0,  1.0,  0.0] },
    Vertex { position: [ 1.0,  1.0, -1.0], normal: [ 0.0,  1.0,  0.0] },
    Vertex { position: [-1.0,  1.0, -1.0], normal: [ 0.0,  1.0,  0.0] },
    // الوجه السفلي (Y-)
    Vertex { position: [-1.0, -1.0, -1.0], normal: [ 0.0, -1.0,  0.0] },
    Vertex { position: [ 1.0, -1.0, -1.0], normal: [ 0.0, -1.0,  0.0] },
    Vertex { position: [ 1.0, -1.0,  1.0], normal: [ 0.0, -1.0,  0.0] },
    Vertex { position: [-1.0, -1.0,  1.0], normal: [ 0.0, -1.0,  0.0] },
    // الوجه الأيمن (X+)
    Vertex { position: [ 1.0, -1.0,  1.0], normal: [ 1.0,  0.0,  0.0] },
    Vertex { position: [ 1.0, -1.0, -1.0], normal: [ 1.0,  0.0,  0.0] },
    Vertex { position: [ 1.0,  1.0, -1.0], normal: [ 1.0,  0.0,  0.0] },
    Vertex { position: [ 1.0,  1.0,  1.0], normal: [ 1.0,  0.0,  0.0] },
    // الوجه الأيسر (X-)
    Vertex { position: [-1.0, -1.0, -1.0], normal: [-1.0,  0.0,  0.0] },
    Vertex { position: [-1.0, -1.0,  1.0], normal: [-1.0,  0.0,  0.0] },
    Vertex { position: [-1.0,  1.0,  1.0], normal: [-1.0,  0.0,  0.0] },
    Vertex { position: [-1.0,  1.0, -1.0], normal: [-1.0,  0.0,  0.0] },
];

const CUBE_INDICES: &[u16] = &[
    0, 1, 2,  0, 2, 3,       // Front
    4, 5, 6,  4, 6, 7,       // Back
    8, 9, 10, 8, 10, 11,     // Top
    12, 13, 14, 12, 14, 15,  // Bottom
    16, 17, 18, 16, 18, 19,  // Right
    20, 21, 22, 20, 22, 23,  // Left
];

const SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.world_normal = model.normal;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.8));
    let n = normalize(in.world_normal);
    let diff = max(dot(n, light_dir), 0.2);
    // تدرج لوني أزرق رمادي مستوحى من مظهر بلندر الافتراضي
    let base_color = vec3<f32>(0.4, 0.5, 0.65);
    return vec4<f32>(base_color * diff, 1.0);
}
"#;

struct RenderState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture_view: wgpu::TextureView,
    
    // بيانات الكاميرا المدارية
    camera_yaw: f32,
    camera_pitch: f32,
    camera_distance: f32,
    is_dragging: bool,
    last_cursor_pos: (f32, f32),
}

impl RenderState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = caps.formats[0];

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        use wgpu::util::DeviceExt;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(CUBE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(CUBE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // إعداد الكاميرا والـ Uniforms
        let camera_yaw: f32 = 0.7;
        let camera_pitch: f32 = 0.5;
        let camera_distance: f32 = 5.0;

        let eye = Vec3::new(
            camera_distance * camera_pitch.cos() * camera_yaw.sin(),
            camera_distance * camera_pitch.sin(),
            camera_distance * camera_pitch.cos() * camera_yaw.cos(),
        );
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
        let aspect = config.width as f32 / config.height as f32;
        let proj = Mat4::perspective_rh(45.0f32.to_radians(), aspect, 0.1, 100.0);
        let view_proj = proj * view;

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(view_proj.to_cols_array().as_slice()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER_SOURCE.into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[ ],
        });

        let depth_texture_view = Self::create_depth_view(&device, &config);

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: CUBE_INDICES.len() as u32,
            camera_buffer,
            camera_bind_group,
            depth_texture_view,
            camera_yaw,
            camera_pitch,
            camera_distance,
            is_dragging: false,
            last_cursor_pos: (0.0, 0.0),
        }
    }

    fn create_depth_view(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };
        let desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[ ],
        };
        let texture = device.create_texture(&desc);
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture_view = Self::create_depth_view(&self.device, &self.config);
            self.update_camera();
        }
    }

    fn update_camera(&mut self) {
        let eye = Vec3::new(
            self.camera_distance * self.camera_pitch.cos() * self.camera_yaw.sin(),
            self.camera_distance * self.camera_pitch.sin(),
            self.camera_distance * self.camera_pitch.cos() * self.camera_yaw.cos(),
        );
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
        let aspect = self.config.width as f32 / self.config.height as f32;
        let proj = Mat4::perspective_rh(45.0f32.to_radians(), aspect, 0.1, 100.0);
        let view_proj = proj * view;

        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(view_proj.to_cols_array().as_slice()),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        // لون خلفية رمادي محايد مثل بيئة بلندر Viewport
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.15, g: 0.16, b: 0.18, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[ ]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        Ok(())
    }
}

#[derive(Default)]
struct App {
    window: Option<Arc<Window>>,
    state: Option<RenderState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            let win_attr = Window::default_attributes().with_title("Rust Mesh Editor");
            let window = Arc::new(event_loop.create_window(win_attr).unwrap());
            self.window = Some(window.clone());

            let state = pollster::block_on(RenderState::new(window.clone()));
            self.state = Some(state);
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let (Some(state), Some(window)) = (self.state.as_mut(), self.window.as_ref()) else {
            return;
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(physical_size) => state.resize(physical_size),
            
            // استقبال ضغط الفأرة واللمس
            WindowEvent::MouseInput { state: element_state, button, .. } => {
                if button == MouseButton::Left || button == MouseButton::Middle {
                    state.is_dragging = element_state == ElementState::Pressed;
                }
            }

            // استقبال حركة المؤشر وتدوير الكاميرا حول المجسم
            WindowEvent::CursorMoved { position, .. } => {
                let (x, y) = (position.x as f32, position.y as f32);
                if state.is_dragging {
                    let dx = x - state.last_cursor_pos.0;
                    let dy = y - state.last_cursor_pos.1;

                    state.camera_yaw += dx * 0.01;
                    state.camera_pitch = (state.camera_pitch + dy * 0.01).clamp(-1.5, 1.5);
                    state.update_camera();
                    window.request_redraw();
                }
                state.last_cursor_pos = (x, y);
            }

            // استقبال التكبير والتصغير عبر عجلة الفأرة (Zoom)
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.05,
                };
                state.camera_distance = (state.camera_distance - scroll * 0.5).clamp(1.5, 30.0);
                state.update_camera();
                window.request_redraw();
            }

            // دعم إيماءات اللمس المباشر على شاشات الهواتف
            WindowEvent::Touch(touch) => {
                let (x, y) = (touch.location.x as f32, touch.location.y as f32);
                match touch.phase {
                    TouchPhase::Started => {
                        state.is_dragging = true;
                        state.last_cursor_pos = (x, y);
                    }
                    TouchPhase::Moved => {
                        if state.is_dragging {
                            let dx = x - state.last_cursor_pos.0;
                            let dy = y - state.last_cursor_pos.1;
                            state.camera_yaw += dx * 0.01;
                            state.camera_pitch = (state.camera_pitch + dy * 0.01).clamp(-1.5, 1.5);
                            state.update_camera();
                            window.request_redraw();
                        }
                        state.last_cursor_pos = (x, y);
                    }
                    TouchPhase::Ended | TouchPhase::Cancelled => {
                        state.is_dragging = false;
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => state.resize(window.inner_size()),
                    Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                    Err(e) => log::warn!("خطأ في عرض الإطار: {:?}", e),
                }
            }
            _ => (),
        }
    }
}

#[no_mangle]
fn android_main(app: AndroidApp) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("RustMeshEditor"),
    );

    let event_loop = EventLoop::builder()
        .with_android_app(app)
        .build()
        .expect("فشل إنشاء حلقة الأحداث");

    let mut application = App::default();
    event_loop.run_app(&mut application).unwrap();
}
