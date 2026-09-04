use std::collections::HashMap;
use std::sync::Arc;
use glam::{Mat4, Vec3};
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    platform::android::activity::AndroidApp,
    platform::android::EventLoopBuilderExtAndroid,
    window::{Window, WindowId},
};

// ==========================================
// 1. هياكل البيانات للرسم ثلاثي الأبعاد والواجهة
// ==========================================

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
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct LineVertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl LineVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct UiVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl UiVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<UiVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 8, shader_location: 1, format: wgpu::VertexFormat::Float32x4 },
            ],
        }
    }
}

// 24 رأس للمكعب الافتراضي
const CUBE_VERTICES: &[Vertex] = &[
    // Front (Z+)
    Vertex { position: [-1.0, -1.0,  1.0], normal: [ 0.0,  0.0,  1.0] },
    Vertex { position: [ 1.0, -1.0,  1.0], normal: [ 0.0,  0.0,  1.0] },
    Vertex { position: [ 1.0,  1.0,  1.0], normal: [ 0.0,  0.0,  1.0] },
    Vertex { position: [-1.0,  1.0,  1.0], normal: [ 0.0,  0.0,  1.0] },
    // Back (Z-)
    Vertex { position: [ 1.0, -1.0, -1.0], normal: [ 0.0,  0.0, -1.0] },
    Vertex { position: [-1.0, -1.0, -1.0], normal: [ 0.0,  0.0, -1.0] },
    Vertex { position: [-1.0,  1.0, -1.0], normal: [ 0.0,  0.0, -1.0] },
    Vertex { position: [ 1.0,  1.0, -1.0], normal: [ 0.0,  0.0, -1.0] },
    // Top (Y+)
    Vertex { position: [-1.0,  1.0,  1.0], normal: [ 0.0,  1.0,  0.0] },
    Vertex { position: [ 1.0,  1.0,  1.0], normal: [ 0.0,  1.0,  0.0] },
    Vertex { position: [ 1.0,  1.0, -1.0], normal: [ 0.0,  1.0,  0.0] },
    Vertex { position: [-1.0,  1.0, -1.0], normal: [ 0.0,  1.0,  0.0] },
    // Bottom (Y-)
    Vertex { position: [-1.0, -1.0, -1.0], normal: [ 0.0, -1.0,  0.0] },
    Vertex { position: [ 1.0, -1.0, -1.0], normal: [ 0.0, -1.0,  0.0] },
    Vertex { position: [ 1.0, -1.0,  1.0], normal: [ 0.0, -1.0,  0.0] },
    Vertex { position: [-1.0, -1.0,  1.0], normal: [ 0.0, -1.0,  0.0] },
    // Right (X+)
    Vertex { position: [ 1.0, -1.0,  1.0], normal: [ 1.0,  0.0,  0.0] },
    Vertex { position: [ 1.0, -1.0, -1.0], normal: [ 1.0,  0.0,  0.0] },
    Vertex { position: [ 1.0,  1.0, -1.0], normal: [ 1.0,  0.0,  0.0] },
    Vertex { position: [ 1.0,  1.0,  1.0], normal: [ 1.0,  0.0,  0.0] },
    // Left (X-)
    Vertex { position: [-1.0, -1.0, -1.0], normal: [-1.0,  0.0,  0.0] },
    Vertex { position: [-1.0, -1.0,  1.0], normal: [-1.0,  0.0,  0.0] },
    Vertex { position: [-1.0,  1.0,  1.0], normal: [-1.0,  0.0,  0.0] },
    Vertex { position: [-1.0,  1.0, -1.0], normal: [-1.0,  0.0,  0.0] },
];

const CUBE_INDICES: &[u16] = &[
    0, 1, 2,  0, 2, 3,
    4, 5, 6,  4, 6, 7,
    8, 9, 10, 8, 10, 11,
    12, 13, 14, 12, 14, 15,
    16, 17, 18, 16, 18, 19,
    20, 21, 22, 20, 22, 23,
];

// ==========================================
// 2. شيدرات الرسوميات (WGSL Shaders)
// ==========================================

const 3D_SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
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
    let world_pos = camera.model * vec4<f32>(model.position, 1.0);
    out.world_normal = (camera.model * vec4<f32>(model.normal, 0.0)).xyz;
    out.clip_position = camera.view_proj * world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.7));
    let n = normalize(in.world_normal);
    let diff = max(dot(n, light_dir), 0.22);
    // تدرج لوني رمادي أزرق يحاكي خامة بلندر الصلبة (Solid MatCap)
    let base_color = vec3<f32>(0.50, 0.54, 0.62);
    return vec4<f32>(base_color * diff, 1.0);
}
"#;

const LINE_SHADER_SOURCE: &str = r#"
struct CameraUniform {
    view_proj: mat4x4<f32>,
    model: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct LineInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

struct LineOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: LineInput) -> LineOutput {
    var out: LineOutput;
    out.color = in.color;
    out.clip_position = camera.view_proj * vec4<f32>(in.position, 1.0);
    return out;
}

@fragment
fn fs_main(in: LineOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

const UI_SHADER_SOURCE: &str = r#"
struct UiInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct UiOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(in: UiInput) -> UiOutput {
    var out: UiOutput;
    out.color = in.color;
    out.clip_position = vec4<f32>(in.position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: UiOutput) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

// ==========================================
// 3. حالة التطبيق ونظام واجهة بلندر
// ==========================================

#[derive(PartialEq, Clone, Copy)]
enum AppMode {
    ObjectMode,
    EditMode,
}

#[derive(PartialEq, Clone, Copy)]
enum ShadingMode {
    Solid,
    Wireframe,
}

#[derive(PartialEq, Clone, Copy)]
enum ActiveTool {
    Select,
    Cursor,
    Move,
    Rotate,
    Scale,
    Extrude,
    LoopCut,
}

struct RenderState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    
    // خطوط الأنابيب الرسومية
    solid_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    ui_pipeline: wgpu::RenderPipeline,

    // مخازن المجسمات
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    grid_buffer: wgpu::Buffer,
    grid_vertex_count: u32,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth_texture_view: wgpu::TextureView,

    // متغيرات الكاميرا والتحكم
    camera_yaw: f32,
    camera_pitch: f32,
    camera_distance: f32,
    is_orbiting: bool,
    is_zooming: bool,
    last_cursor_pos: (f32, f32),
    active_touches: HashMap<u64, (f32, f32)>,
    last_pinch_distance: Option<f32>,

    // حالات بلندر التفاعلية
    mode: AppMode,
    shading: ShadingMode,
    active_tool: ActiveTool,
    cube_pos: Vec3,
    cube_rot: Vec3,
    cube_scale: Vec3,
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
            label: Some("Cube Vertex Buffer"),
            contents: bytemuck::cast_slice(CUBE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Index Buffer"),
            contents: bytemuck::cast_slice(CUBE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // توليد أرضية شبكة الإحداثيات (Blender Floor Grid)
        let mut grid_lines: Vec<LineVertex> = Vec::new();
        let grid_size = 8;
        let grid_color = [0.24, 0.26, 0.30, 0.7];
        let x_axis_color = [0.85, 0.22, 0.22, 1.0]; // أحمر للمحور X
        let z_axis_color = [0.22, 0.80, 0.25, 1.0]; // أخضر للمحور Z

        for i in -grid_size..=grid_size {
            let fi = i as f32;
            let f_size = grid_size as f32;

            // خطوط موازية لـ X
            let col = if i == 0 { x_axis_color } else { grid_color };
            grid_lines.push(LineVertex { position: [-f_size, -1.0, fi], color: col });
            grid_lines.push(LineVertex { position: [ f_size, -1.0, fi], color: col });

            // خطوط موازية لـ Z
            let col_z = if i == 0 { z_axis_color } else { grid_color };
            grid_lines.push(LineVertex { position: [fi, -1.0, -f_size], color: col_z });
            grid_lines.push(LineVertex { position: [fi, -1.0,  f_size], color: col_z });
        }

        let grid_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Buffer"),
            contents: bytemuck::cast_slice(&grid_lines),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let grid_vertex_count = grid_lines.len() as u32;

        let camera_yaw: f32 = 0.75;
        let camera_pitch: f32 = 0.50;
        let camera_distance: f32 = 7.0;

        let eye = Vec3::new(
            camera_distance * camera_pitch.cos() * camera_yaw.sin(),
            camera_distance * camera_pitch.sin(),
            camera_distance * camera_pitch.cos() * camera_yaw.cos(),
        );
        let view = Mat4::look_at_rh(eye, Vec3::ZERO, Vec3::Y);
        let aspect = config.width as f32 / config.height as f32;
        let proj = Mat4::perspective_rh(45.0f32.to_radians(), aspect, 0.1, 100.0);
        let view_proj = proj * view;
        let model = Mat4::IDENTITY;

        let mut uniform_data = Vec::new();
        uniform_data.extend_from_slice(view_proj.to_cols_array().as_slice());
        uniform_data.extend_from_slice(model.to_cols_array().as_slice());

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&uniform_data),
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
            label: Some("Camera Layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() }],
            label: Some("Camera Bind Group"),
        });

        // إعداد الشيدرات
        let solid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Solid Shader"),
            source: wgpu::ShaderSource::Wgsl(3D_SHADER_SOURCE.into()),
        });
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(LINE_SHADER_SOURCE.into()),
        });
        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI Shader"),
            source: wgpu::ShaderSource::Wgsl(UI_SHADER_SOURCE.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[ ],
        });

        let depth_texture_view = Self::create_depth_view(&device, &config);

        // خط أنابيب المجسمات المصمتة
        let solid_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Solid Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &solid_shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &solid_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // خط أنابيب شبكة الإحداثيات والخطوط
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                buffers: &[LineVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        // خط أنابيب واجهة بلندر (UI 2D Pipeline)
        let ui_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("UI Layout"),
            bind_group_layouts: &[ ],
            push_constant_ranges: &[ ],
        });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Pipeline"),
            layout: Some(&ui_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs_main"),
                buffers: &[UiVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });

        Self {
            surface,
            device,
            queue,
            config,
            solid_pipeline,
            line_pipeline,
            ui_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices: CUBE_INDICES.len() as u32,
            grid_buffer,
            grid_vertex_count,
            camera_buffer,
            camera_bind_group,
            depth_texture_view,
            camera_yaw,
            camera_pitch,
            camera_distance,
            is_orbiting: false,
            is_zooming: false,
            last_cursor_pos: (0.0, 0.0),
            active_touches: HashMap::new(),
            last_pinch_distance: None,
            mode: AppMode::ObjectMode,
            shading: ShadingMode::Solid,
            active_tool: ActiveTool::Select,
            cube_pos: Vec3::ZERO,
            cube_rot: Vec3::ZERO,
            cube_scale: Vec3::ONE,
        }
    }

    fn create_depth_view(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_la
