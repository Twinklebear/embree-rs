use core::num::NonZeroU32;
use std::{arch::x86_64::_rdtsc, borrow::Cow, fmt::Debug};

use crate::{rgba_to_u32, Camera, DebugState, ShadingMode, TiledImage, TILE_SIZE_X, TILE_SIZE_Y};
use arcball::ArcballCamera;
use cgmath::{InnerSpace, Vector2, Vector3};
use clock_ticks;
use egui_wgpu::renderer::ScreenDescriptor;
use embree::{IntersectContext, Ray, RayHit, RayHitNp, RayNp};
use futures;
use rayon::iter::ParallelIterator;
use wgpu;
use winit::{
    dpi::{LogicalSize, Size},
    event::{
        ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
        WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop, EventLoopBuilder},
    window::{Window, WindowBuilder},
};

const WGSL_SHADERS: &str = "
type float2 = vec2<f32>;
type float4 = vec4<f32>;
type int2 = vec2<i32>;

struct VertexInput {
    @builtin(vertex_index) index: u32,
};

struct VertexOutput {
    @builtin(position) position: float4,
};

var<private> coords: array<float2, 4> = array<float2, 4>(
    float2(-1.0, -1.0),
    float2(1.0, -1.0),
    float2(-1.0, 1.0),
    float2(1.0, 1.0)
);

@vertex
fn vertex_main(vert: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = float4(coords[vert.index], 0.0, 1.0);
    return out;
};

@group(0) @binding(0)
var image: texture_2d<f32>;

@fragment
fn fragment_main(in: VertexOutput) -> @location(0) float4 {
    return textureLoad(image, int2(in.position.xy), 0);
}
";

/// Manager to display the rendered image in an interactive window.
pub struct Display {
    window: Window,
    event_loop: EventLoop<()>,
    #[allow(dead_code)]
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    #[allow(dead_code)]
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
        let event_loop = EventLoopBuilder::<()>::new().build();
        let win_size = Size::Logical(LogicalSize::new(w as f64, h as f64));
        let window = WindowBuilder::new()
            .with_inner_size(win_size)
            .with_title(title)
            .build(&event_loop)
            .unwrap();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            dx12_shader_compiler: Default::default(),
        });
        let surface =
            unsafe { instance.create_surface(&window) }.expect("Failed to create surface");
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
            window,
            event_loop,
            instance,
            surface,
            adapter,
            device,
            queue,
        }
    }
}

/// The function passed should render and update the image to be displayed in
/// the window, optionally using the camera pose information passed.
pub fn run<F, G, U, T>(
    display: Display,
    mut state: DebugState<T>,
    mut update: G,
    mut render: F,
    run_ui: U,
) where
    F: FnMut(&mut TiledImage, &Camera, f32, &mut DebugState<T>) + 'static,
    G: FnMut(f32, &mut DebugState<T>) + 'static,
    U: FnOnce(&egui::Context) + Copy + 'static,
    T: Sized + Send + Sync + 'static,
{
    let mut window_size = display.window.inner_size();
    let mut image_buf: Vec<u8> = vec![0u8; (window_size.width * window_size.height * 4) as usize];

    let mut embree_target = TiledImage::new(
        window_size.width,
        window_size.height,
        TILE_SIZE_X,
        TILE_SIZE_Y,
    );

    let mut arcball = ArcballCamera::new(
        Vector3::new(0.0, 0.0, 0.0),
        1.0,
        [window_size.width as f32, window_size.height as f32],
    );
    arcball.zoom(-30.0, 0.16);
    arcball.rotate(
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
        .create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(WGSL_SHADERS)),
        });
    let fragment_module = display
        .device
        .create_shader_module(wgpu::ShaderModuleDescriptor {
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

    let mut window_extent = wgpu::Extent3d {
        width: window_size.width,
        height: window_size.height,
        depth_or_array_layers: 1,
    };

    let mut embree_texture = display.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: window_extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
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

    let mut bind_group = display
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
            present_mode: wgpu::PresentMode::AutoNoVsync,
            alpha_mode: Default::default(),
            view_formats: vec![],
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
                targets: &[Some(wgpu::ColorTargetState {
                    format: swap_chain_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

    let clear_color = wgpu::Color {
        r: 0.3,
        g: 0.3,
        b: 0.3,
        a: 1.0,
    };

    let egui_ctx = egui::Context::default();
    let mut egui_state = egui_winit::State::new(&display.event_loop);
    let mut egui_renderer = egui_wgpu::Renderer::new(&display.device, swap_chain_format, None, 1);

    let mut screen_desc = ScreenDescriptor {
        size_in_pixels: window_size.into(),
        pixels_per_point: display.window.scale_factor() as f32,
    };

    let mut shading_mode = ShadingMode::Default;
    let mut fps = 0.0f64;
    let mut mouse_prev = Vector2::new(0.0, 0.0);
    let mut mouse_pressed = [false, false, false];
    let t_start = clock_ticks::precise_time_s();
    let mut last_frame_time = t_start;

    display.event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event, .. } => {
                if !egui_state.on_event(&egui_ctx, &event).consumed {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput { input, .. } => match input {
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } => *control_flow = ControlFlow::Exit,
                            _ => {}
                        },
                        WindowEvent::MouseInput { state, button, .. } => match button {
                            MouseButton::Left => mouse_pressed[0] = state == ElementState::Pressed,
                            MouseButton::Middle => {
                                mouse_pressed[1] = state == ElementState::Pressed
                            }
                            MouseButton::Right => mouse_pressed[2] = state == ElementState::Pressed,
                            MouseButton::Other(_) => {}
                        },
                        WindowEvent::CursorMoved { position, .. } => {
                            let mouse_cur = Vector2::new(position.x as f32, position.y as f32);
                            if mouse_pressed[0] {
                                arcball.rotate(mouse_prev, mouse_cur);
                            }
                            if mouse_pressed[2] {
                                arcball.pan(mouse_cur - mouse_prev);
                            }
                            mouse_prev = mouse_cur;
                        }
                        WindowEvent::MouseWheel { delta, .. } => match delta {
                            MouseScrollDelta::LineDelta(_, y) => {
                                arcball.zoom(y, 0.1);
                            }
                            MouseScrollDelta::PixelDelta(pos) => {
                                arcball.zoom(pos.y as f32, 0.01);
                            }
                        },
                        WindowEvent::Resized(size)
                        | WindowEvent::ScaleFactorChanged {
                            new_inner_size: &mut size,
                            ..
                        } => {
                            if size.width > 0 && size.height > 0 {
                                if size.width != window_size.width
                                    || size.height != window_size.height
                                {
                                    window_size = size;

                                    // update arcball
                                    arcball.update_screen(
                                        window_size.width as f32,
                                        window_size.height as f32,
                                    );

                                    // update swapchain
                                    display.surface.configure(
                                        &display.device,
                                        &wgpu::SurfaceConfiguration {
                                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                                            format: swap_chain_format,
                                            width: window_size.width,
                                            height: window_size.height,
                                            present_mode: wgpu::PresentMode::AutoNoVsync,
                                            alpha_mode: Default::default(),
                                            view_formats: vec![],
                                        },
                                    );

                                    image_buf.resize(
                                        (window_size.width * window_size.height * 4) as usize,
                                        0,
                                    );

                                    // update embree target
                                    embree_target = TiledImage::new(
                                        window_size.width as u32,
                                        window_size.height as u32,
                                        TILE_SIZE_X,
                                        TILE_SIZE_Y,
                                    );
                                    window_extent = wgpu::Extent3d {
                                        width: window_size.width,
                                        height: window_size.height,
                                        depth_or_array_layers: 1,
                                    };
                                    // recreate embree texture
                                    embree_texture =
                                        display.device.create_texture(&wgpu::TextureDescriptor {
                                            label: None,
                                            size: window_extent,
                                            mip_level_count: 1,
                                            sample_count: 1,
                                            dimension: wgpu::TextureDimension::D2,
                                            format: wgpu::TextureFormat::Rgba8Unorm,
                                            usage: wgpu::TextureUsages::COPY_DST
                                                | wgpu::TextureUsages::TEXTURE_BINDING,
                                            view_formats: &[],
                                        });
                                    // update screen size for egui
                                    screen_desc.size_in_pixels = window_size.into();
                                    screen_desc.pixels_per_point =
                                        display.window.scale_factor() as f32;

                                    bind_group = display.device.create_bind_group(
                                        &wgpu::BindGroupDescriptor {
                                            label: None,
                                            layout: &bindgroup_layout,
                                            entries: &[wgpu::BindGroupEntry {
                                                binding: 0,
                                                resource: wgpu::BindingResource::TextureView(
                                                    &embree_texture.create_view(
                                                        &wgpu::TextureViewDescriptor {
                                                            label: None,
                                                            format: Some(
                                                                wgpu::TextureFormat::Rgba8Unorm,
                                                            ),
                                                            dimension: Some(
                                                                wgpu::TextureViewDimension::D2,
                                                            ),
                                                            aspect: wgpu::TextureAspect::All,
                                                            base_mip_level: 0,
                                                            mip_level_count: None,
                                                            base_array_layer: 0,
                                                            array_layer_count: None,
                                                        },
                                                    ),
                                                ),
                                            }],
                                        },
                                    );
                                }
                            }
                        }
                        _ => (),
                    }
                }
            }
            Event::MainEventsCleared => {
                let egui_input = egui_state.take_egui_input(&display.window);

                let cam_pose =
                    CameraPose::new(arcball.eye_pos(), arcball.eye_dir(), arcball.up_dir());

                let camera = Camera::look_dir(
                    cam_pose.pos,
                    cam_pose.dir,
                    cam_pose.up,
                    75.0,
                    (window_size.width, window_size.height),
                );

                let time = (clock_ticks::precise_time_s() - t_start) as f32;

                update(time, &mut state);

                // render embree target
                embree_target.reset_pixels();
                match shading_mode {
                    ShadingMode::Default => {
                        render(
                            &mut embree_target,
                            &camera,
                            (clock_ticks::precise_time_s() - t_start) as f32,
                            &mut state,
                        );
                    }
                    ShadingMode::EyeLight => {
                        render_frame_eye_light(
                            &mut embree_target,
                            (clock_ticks::precise_time_s() - t_start) as f32,
                            &camera,
                            &state,
                        );
                    }
                    ShadingMode::Occlusion => {}
                    ShadingMode::UV => {
                        render_frame_pixel_uv(
                            &mut embree_target,
                            (clock_ticks::precise_time_s() - t_start) as f32,
                            &camera,
                            &state,
                        );
                    }
                    ShadingMode::Normal => {
                        render_frame_pixel_normal(
                            &mut embree_target,
                            (clock_ticks::precise_time_s() - t_start) as f32,
                            &camera,
                            &state,
                        );
                    }
                    ShadingMode::CPUCycles => {
                        render_frame_pixel_cpu_cycles(
                            &mut embree_target,
                            (clock_ticks::precise_time_s() - t_start) as f32,
                            &camera,
                            &state,
                        );
                    }
                    ShadingMode::GeometryID => {
                        render_frame_pixel_geometry_id(
                            &mut embree_target,
                            (clock_ticks::precise_time_s() - t_start) as f32,
                            &camera,
                            &state,
                        );
                    }
                    ShadingMode::GeometryPrimitiveID => {
                        render_frame_pixel_geometry_primitive_id(
                            &mut embree_target,
                            (clock_ticks::precise_time_s() - t_start) as f32,
                            &camera,
                            &state,
                        );
                    }
                    // TODO(yang): implement
                    ShadingMode::AmbientOcclusion
                    | ShadingMode::TexCoords
                    | ShadingMode::TexCoordsGrid => {
                        render(
                            &mut embree_target,
                            &camera,
                            (clock_ticks::precise_time_s() - t_start) as f32,
                            &mut state,
                        );
                    }
                }
                embree_target.write_to_flat_buffer(&mut image_buf);

                // Just use queue write_texture even though it likely makes a temporary upload
                // buffer, because making the async map API work in here will be a mess.
                display.queue.write_texture(
                    embree_texture.as_image_copy(),
                    &image_buf,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some(NonZeroU32::new(window_size.width * 4).unwrap()),
                        rows_per_image: Some(NonZeroU32::new(window_size.height).unwrap()),
                    },
                    window_extent,
                );

                // present embree target on screen
                let frame = display
                    .surface
                    .get_current_texture()
                    .expect("Failed to get surface output texture");
                let render_target_view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = display
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &render_target_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(clear_color),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    render_pass.set_pipeline(&render_pipeline);
                    render_pass.set_bind_group(0, &bind_group, &[]);
                    // Note: also bug in wgpu-rs set_index_buffer or web sys not passing
                    // the right index type
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..4, 0, 0..1);
                }

                let mut ui_encoder =
                    display
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("egui_encoder"),
                        });
                {
                    let egui_output = egui_ctx.run(egui_input, |ctx: &egui::Context| {
                        run_ui(ctx);
                        egui::Window::new("fps")
                            .title_bar(false)
                            .min_width(200.0)
                            .show(ctx, |ui| {
                                ui.vertical(|ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label("FPS: ");
                                        ui.label(format!("{:3}", fps.floor()));
                                    });

                                    ui.horizontal_wrapped(|ui| {
                                        ui.label("Shading: ");
                                        egui::ComboBox::from_label("")
                                            .selected_text(format!("{:?}", shading_mode))
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::Default,
                                                    "Default",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::EyeLight,
                                                    "EyeLight",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::Normal,
                                                    "Normal",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::CPUCycles,
                                                    "CpuCycles",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::GeometryID,
                                                    "GeomID",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::GeometryPrimitiveID,
                                                    "GeomPrimID",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::UV,
                                                    "Uv",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::Occlusion,
                                                    "Occlusion",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::TexCoords,
                                                    "TexCoords",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::TexCoordsGrid,
                                                    "TexCoordsGrid",
                                                );
                                                ui.selectable_value(
                                                    &mut shading_mode,
                                                    ShadingMode::AmbientOcclusion,
                                                    "AmbientOcclusion",
                                                );
                                            });
                                    });
                                });
                            });
                    });
                    egui_state.handle_platform_output(
                        &display.window,
                        &egui_ctx,
                        egui_output.platform_output,
                    );
                    let primitives = egui_ctx.tessellate(egui_output.shapes);
                    let _user_cmds = {
                        for (id, image_delta) in &egui_output.textures_delta.set {
                            egui_renderer.update_texture(
                                &display.device,
                                &display.queue,
                                *id,
                                image_delta,
                            );
                        }
                        egui_renderer.update_buffers(
                            &display.device,
                            &display.queue,
                            &mut ui_encoder,
                            &primitives,
                            &screen_desc,
                        )
                    };
                    {
                        let mut render_pass =
                            ui_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                label: Some("egui_render_pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &render_target_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Load,
                                        store: true,
                                    },
                                })],
                                depth_stencil_attachment: None,
                            });

                        egui_renderer.render(&mut render_pass, &primitives, &screen_desc);
                    }

                    for id in &egui_output.textures_delta.free {
                        egui_renderer.free_texture(id);
                    }
                }

                display
                    .queue
                    .submit([encoder.finish(), ui_encoder.finish()]);
                frame.present();

                let elapsed = clock_ticks::precise_time_s() - last_frame_time;
                last_frame_time = clock_ticks::precise_time_s();
                fps = 1.0 / elapsed;
            }
            _ => (),
        }
    });
}

fn render_frame_eye_light<T: Sized + Send + Sync>(
    frame: &mut TiledImage,
    _time: f32,
    camera: &Camera,
    state: &DebugState<T>,
) {
    frame.par_tiles_mut().for_each(|tile| {
        let tile_size = (tile.w * tile.h) as usize;
        let mut ray_hits = RayHitNp::new(RayNp::new(tile_size));
        for (i, mut ray) in ray_hits.ray.iter_mut().enumerate() {
            let x = tile.x + (i % tile.w as usize) as u32;
            let y = tile.y + (i / tile.w as usize) as u32;
            ray.set_origin(camera.pos.into());
            ray.set_dir(camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5)).into());
            ray.set_tnear(0.0);
            ray.set_tfar(f32::INFINITY);
        }
        let mut ctx = IntersectContext::coherent();
        state.scene.intersect_stream_soa(&mut ctx, &mut ray_hits);

        for (i, (ray, hit)) in ray_hits.iter().enumerate() {
            if hit.is_valid() {
                let dot = Vector3::from(hit.unit_normal()).dot(Vector3::from(ray.unit_dir()));
                if dot < 0.0 {
                    tile.pixels[i] = rgba_to_u32(0, (dot.abs() * 255.0) as u8, 0, 255);
                } else {
                    tile.pixels[i] = rgba_to_u32((dot.abs() * 255.0) as u8, 0, 0, 255);
                }
            } else {
                tile.pixels[i] = rgba_to_u32(0, 0, 0, 255);
            }
        }
    });
}

fn render_frame_pixel_uv<T: Sized + Send + Sync>(
    frame: &mut TiledImage,
    _time: f32,
    camera: &Camera,
    state: &DebugState<T>,
) {
    frame.par_tiles_mut().for_each(|tile| {
        let tile_size = (tile.w * tile.h) as usize;
        let mut ray_hits = RayHitNp::new(RayNp::new(tile_size));
        for (i, mut ray) in ray_hits.ray.iter_mut().enumerate() {
            let x = tile.x + (i % tile.w as usize) as u32;
            let y = tile.y + (i / tile.w as usize) as u32;
            ray.set_origin(camera.pos.into());
            ray.set_dir(camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5)).into());
            ray.set_tnear(0.0);
            ray.set_tfar(f32::INFINITY);
        }
        let mut ctx = IntersectContext::coherent();
        state.scene.intersect_stream_soa(&mut ctx, &mut ray_hits);

        for (i, (_, hit)) in ray_hits.iter().enumerate() {
            if hit.is_valid() {
                let [u, v] = hit.uv();
                tile.pixels[i] = rgba_to_u32(
                    (u * 255.0) as u8,
                    (v * 255.0) as u8,
                    ((1.0 - u - v) * 255.0) as u8,
                    255,
                );
            } else {
                tile.pixels[i] = rgba_to_u32(0, 0, 255, 255);
            }
        }
    });
}

fn render_frame_pixel_normal<T: Sized + Send + Sync>(
    frame: &mut TiledImage,
    _time: f32,
    camera: &Camera,
    state: &DebugState<T>,
) {
    frame.par_tiles_mut().for_each(|tile| {
        let tile_size = (tile.w * tile.h) as usize;
        let mut ray_hits = RayHitNp::new(RayNp::new(tile_size));
        for (i, mut ray) in ray_hits.ray.iter_mut().enumerate() {
            let x = tile.x + (i % tile.w as usize) as u32;
            let y = tile.y + (i / tile.w as usize) as u32;
            ray.set_origin(camera.pos.into());
            ray.set_dir(camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5)).into());
            ray.set_tnear(0.0);
            ray.set_tfar(f32::INFINITY);
        }
        let mut ctx = IntersectContext::coherent();
        state.scene.intersect_stream_soa(&mut ctx, &mut ray_hits);

        for (i, (_, hit)) in ray_hits.iter().enumerate() {
            if hit.is_valid() {
                let [nx, ny, nz] = hit.unit_normal();
                tile.pixels[i] = rgba_to_u32(
                    (nx.abs() * 255.0) as u8,
                    (ny.abs() * 255.0) as u8,
                    (nz.abs() * 255.0) as u8,
                    255,
                );
            } else {
                tile.pixels[i] = rgba_to_u32(0, 0, 255, 255);
            }
        }
    });
}

fn render_frame_pixel_geometry_id<T: Sized + Send + Sync>(
    frame: &mut TiledImage,
    _time: f32,
    camera: &Camera,
    state: &DebugState<T>,
) {
    frame.par_tiles_mut().for_each(|tile| {
        let tile_size = (tile.w * tile.h) as usize;
        let mut ray_hits = RayHitNp::new(RayNp::new(tile_size));
        for (i, mut ray) in ray_hits.ray.iter_mut().enumerate() {
            let x = tile.x + (i % tile.w as usize) as u32;
            let y = tile.y + (i / tile.w as usize) as u32;
            ray.set_origin(camera.pos.into());
            ray.set_dir(camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5)).into());
            ray.set_tnear(0.0);
            ray.set_tfar(f32::INFINITY);
        }
        let mut ctx = IntersectContext::coherent();
        state.scene.intersect_stream_soa(&mut ctx, &mut ray_hits);

        for (i, (_, hit)) in ray_hits.iter().enumerate() {
            if hit.is_valid() {
                let geom_id = hit.geom_id();
                let [r, g, b] = random_color(geom_id);
                tile.pixels[i] = rgba_to_u32(r, g, b, 255);
            } else {
                tile.pixels[i] = rgba_to_u32(0, 0, 0, 255);
            }
        }
    });
}

fn random_color(id: u32) -> [u8; 3] {
    [
        (((id + 13) * 17 * 23) & 255) as u8,
        (((id + 15) * 11 * 13) & 255) as u8,
        (((id + 17) * 7 * 19) & 255) as u8,
    ]
}

fn random_color_f32(id: u32) -> [f32; 3] {
    let one_over_255 = 1.0 / 255.0;
    [
        (((id + 13) * 17 * 23) & 255) as f32 * one_over_255,
        (((id + 15) * 11 * 13) & 255) as f32 * one_over_255,
        (((id + 17) * 7 * 19) & 255) as f32 * one_over_255,
    ]
}

fn render_frame_pixel_cpu_cycles<T: Sized + Send + Sync>(
    frame: &mut TiledImage,
    _time: f32,
    camera: &Camera,
    state: &DebugState<T>,
) {
    frame.par_tiles_mut().for_each(|tile| {
        for (i, pixel) in tile.pixels.iter_mut().enumerate() {
            let x = tile.x + (i % tile.w as usize) as u32;
            let y = tile.y + (i / tile.w as usize) as u32;

            let mut ray_hit = RayHit::from_ray(Ray::segment(
                camera.pos.into(),
                camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5)).into(),
                0.0,
                f32::INFINITY,
            ));

            let c0 = unsafe { _rdtsc() };
            let mut ctx = IntersectContext::coherent();
            state.scene.intersect(&mut ctx, &mut ray_hit);
            let c1 = unsafe { _rdtsc() };
            *pixel = rgba_to_u32(
                ((c1 - c0) & 255) as u8,
                ((c1 - c0) >> 8 & 255) as u8,
                ((c1 - c0) >> 16 & 255) as u8,
                255,
            );
        }
    });
}

fn render_frame_pixel_geometry_primitive_id<T: Sized + Send + Sync>(
    frame: &mut TiledImage,
    _time: f32,
    camera: &Camera,
    state: &DebugState<T>,
) {
    frame.par_tiles_mut().for_each(|tile| {
        let tile_size = (tile.w * tile.h) as usize;
        let mut ray_hits = RayHitNp::new(RayNp::new(tile_size));
        for (i, mut ray) in ray_hits.ray.iter_mut().enumerate() {
            let x = tile.x + (i % tile.w as usize) as u32;
            let y = tile.y + (i / tile.w as usize) as u32;
            ray.set_origin(camera.pos.into());
            ray.set_dir(camera.ray_dir((x as f32 + 0.5, y as f32 + 0.5)).into());
            ray.set_tnear(0.0);
            ray.set_tfar(f32::INFINITY);
        }
        let mut ctx = IntersectContext::coherent();
        state.scene.intersect_stream_soa(&mut ctx, &mut ray_hits);

        for (i, (ray, hit)) in ray_hits.iter().enumerate() {
            if hit.is_valid() {
                let geom_id = hit.geom_id();
                let prim_id = hit.prim_id();
                let [r, g, b] = random_color_f32(geom_id ^ prim_id);
                let dot = (Vector3::from(hit.unit_normal()).dot(Vector3::from(ray.dir()))).abs();
                tile.pixels[i] = rgba_to_u32(
                    (r * dot * 255.0) as u8,
                    (g * dot * 255.0) as u8,
                    (b * dot * 255.0) as u8,
                    255,
                );
            } else {
                tile.pixels[i] = rgba_to_u32(0, 0, 0, 255);
            }
        }
    });
}
