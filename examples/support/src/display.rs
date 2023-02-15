use core::num::NonZeroU32;
use std::borrow::Cow;

use arcball::ArcballCamera;
use cgmath::{Matrix4, SquareMatrix, Vector2, Vector3, Vector4};
use clock_ticks;
use futures;
use image::RgbaImage;
use wgpu;
use winit::{
    dpi::{LogicalSize, Size},
    event::{ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};

const WGSL_SHADERS: &str = "
type float2 = vec2<f32>;
type float4 = vec4<f32>;
type int2 = vec2<i32>;

struct VertexInput {
    [[builtin(vertex_index)]] index: u32;
};

struct VertexOutput {
    [[builtin(position)]] position: float4;
};

var<private> coords: array<float2, 4> = array<float2, 4>(
    float2(-1.0, -1.0),
    float2(1.0, -1.0),
    float2(-1.0, 1.0),
    float2(1.0, 1.0)
);

[[stage(vertex)]]
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = float4(coords[vert.index], 0.0, 1.0);
    return out;
};

[[group(0), binding(0)]]
var image: texture_2d<f32>;

[[stage(fragment)]]
fn fragment_main(in: VertexOutput) -> [[location(0)]] float4 {
    return textureLoad(image, int2(in.position.xy), 0);
}
";

/// Manager to display the rendered image in an interactive window.
pub struct Display {
    window: Window,
    event_loop: EventLoop<()>,
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
}

#[derive(Debug)]
pub struct CameraPose {
    pub pos: Vector3<f32>,
    pub dir: Vector3<f32>,
    pub up: Vector3<f32>,
}
impl CameraPose {
    fn new(pos: Vector3<f32>, dir: Vector3<f32>, up: Vector3<f32>) -> CameraPose {
        CameraPose { pos, dir, up }
    }
}

impl Display {
    pub fn new(w: u32, h: u32, title: &str) -> Display {
        let event_loop = EventLoop::new();
        let win_size = Size::Logical(LogicalSize::new(w as f64, h as f64));
        let window = WindowBuilder::new()
            .with_inner_size(win_size)
            .with_title(title)
            .build(&event_loop)
            .unwrap();

        let instance = wgpu::Instance::new(wgpu::Backends::PRIMARY);
        let surface = unsafe { instance.create_surface(&window) };
        let adapter =
            futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }))
            .expect("Failed to find a WebGPU adapter");

        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        ))
        .expect("Failed to create device");

        Display {
            window: window,
            event_loop: event_loop,
            instance: instance,
            surface: surface,
            adapter: adapter,
            device: device,
            queue: queue,
        }
    }
}
/// The function passed should render and update the image to be displayed in
/// the window, optionally using the camera pose information passed.
pub fn run<F>(display: Display, mut render: F)
where
    F: 'static + FnMut(&mut RgbaImage, CameraPose, f32),
{
    let window_size = display.window.inner_size();
    let mut embree_target = RgbaImage::new(window_size.width, window_size.height);

    let mut arcball_camera = ArcballCamera::new(
        Vector3::new(0.0, 0.0, 0.0),
        1.0,
        [window_size.width as f32, window_size.height as f32],
    );
    arcball_camera.zoom(-50.0, 0.16);
    arcball_camera.rotate(
        Vector2::new(
            window_size.width as f32 / 2.0,
            window_size.height as f32 / 4.0,
        ),
        Vector2::new(
            window_size.width as f32 / 2.0,
            window_size.height as f32 / 3.0,
        ),
    );

    // Porting in my wgpu-rs example just to test set up
    let vertex_module = display
        .device
        .create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL_SHADERS)),
        });
    let fragment_module = display
        .device
        .create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL_SHADERS)),
        });

    let index_data: [u16; 4] = [0, 1, 2, 3];
    let index_buffer = display.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (index_data.len() * 4) as u64,
        usage: wgpu::BufferUsages::INDEX,
        mapped_at_creation: true,
    });
    {
        let mut view = index_buffer.slice(..).get_mapped_range_mut();
        let u16_view = unsafe {
            std::slice::from_raw_parts_mut(view.as_mut_ptr() as *mut u16, index_data.len())
        };
        u16_view.copy_from_slice(&index_data)
    }
    index_buffer.unmap();

    let window_extent = wgpu::Extent3d {
        width: window_size.width,
        height: window_size.height,
        depth_or_array_layers: 1,
    };
    let embree_texture = display.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: window_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
    });

    let bindgroup_layout =
        display
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                }],
            });

    let bindgroup = display
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bindgroup_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&embree_texture.create_view(
                    &wgpu::TextureViewDescriptor {
                        label: None,
                        format: Some(wgpu::TextureFormat::Rgba8Unorm),
                        dimension: Some(wgpu::TextureViewDimension::D2),
                        aspect: wgpu::TextureAspect::All,
                        base_mip_level: 0,
                        mip_level_count: None,
                        base_array_layer: 0,
                        array_layer_count: None,
                    },
                )),
            }],
        });

    let pipeline_layout = display
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bindgroup_layout],
            push_constant_ranges: &[],
        });

    let swap_chain_format = wgpu::TextureFormat::Bgra8Unorm;

    display.surface.configure(
        &display.device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swap_chain_format,
            width: window_size.width,
            height: window_size.height,
            present_mode: wgpu::PresentMode::Fifo,
        },
    );

    let render_pipeline = display
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vertex_module,
                entry_point: "vertex_main",
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                // Note: it's not possible to set a "none" strip index format,
                // which raises an error in Chrome Canary b/c when using non-strip
                // topologies, the index format must be none. However, wgpu-rs
                // instead defaults this to uint16, leading to an invalid state.
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &fragment_module,
                entry_point: "fragment_main",
                targets: &[wgpu::ColorTargetState {
                    format: swap_chain_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            multiview: None,
        });

    let clear_color = wgpu::Color {
        r: 0.3,
        g: 0.3,
        b: 0.3,
        a: 1.0,
    };

    let mut mouse_prev = Vector2::new(0.0, 0.0);
    let mut mouse_pressed = [false, false, false];
    let t_start = clock_ticks::precise_time_s();
    display.event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. }
                    if input.virtual_keycode == Some(VirtualKeyCode::Escape) =>
                {
                    *control_flow = ControlFlow::Exit
                }
                WindowEvent::MouseInput { state, button, .. } => match button {
                    MouseButton::Left => mouse_pressed[0] = state == ElementState::Pressed,
                    MouseButton::Middle => mouse_pressed[1] = state == ElementState::Pressed,
                    MouseButton::Right => mouse_pressed[2] = state == ElementState::Pressed,
                    MouseButton::Other(_) => {}
                },
                WindowEvent::CursorMoved { position, .. } => {
                    let mouse_cur = Vector2::new(position.x as f32, position.y as f32);
                    if mouse_pressed[0] {
                        arcball_camera.rotate(mouse_prev, mouse_cur);
                    }
                    if mouse_pressed[2] {
                        arcball_camera.pan(mouse_cur - mouse_prev);
                    }
                    mouse_prev = mouse_cur;
                }
                WindowEvent::MouseWheel { delta, .. } => match delta {
                    MouseScrollDelta::LineDelta(_, y) => {
                        arcball_camera.zoom(y, 0.1);
                    }
                    MouseScrollDelta::PixelDelta(pos) => {
                        arcball_camera.zoom(pos.y as f32, 0.01);
                    }
                },
                _ => (),
            },
            Event::MainEventsCleared => {
                let cam_pose = CameraPose::new(
                    arcball_camera.eye_pos(),
                    arcball_camera.eye_dir(),
                    arcball_camera.up_dir(),
                );
                render(
                    &mut embree_target,
                    cam_pose,
                    (clock_ticks::precise_time_s() - t_start) as f32,
                );

                let frame = display
                    .surface
                    .get_current_texture()
                    .expect("Failed to get surface output texture");
                let render_target_view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                // Just use queue write_texture even though it likely makes a temporary upload
                // buffer, because making the async map API work in here will be a mess.
                display.queue.write_texture(
                    embree_texture.as_image_copy(),
                    &embree_target.as_raw()[..],
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(NonZeroU32::new(window_size.width * 4).unwrap()),
                        rows_per_image: Some(NonZeroU32::new(window_size.height).unwrap()),
                    },
                    window_extent,
                );

                let mut encoder = display
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[wgpu::RenderPassColorAttachment {
                            view: &render_target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(clear_color),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

                    render_pass.set_pipeline(&render_pipeline);
                    render_pass.set_bind_group(0, &bindgroup, &[]);
                    // Note: also bug in wgpu-rs set_index_buffer or web sys not passing
                    // the right index type
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..4, 0, 0..1);
                }
                display.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            _ => (),
        }
    });
}
