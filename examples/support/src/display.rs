use glium::{self, glutin, Surface};
use glium::glutin::{Event, WindowEvent, VirtualKeyCode, ElementState, MouseScrollDelta, MouseButton};
use glium::texture::RawImage2d;
use glium::Texture2d;
use image::RgbImage;
use arcball::ArcballCamera;
use cgmath::{self, SquareMatrix};

use vec3f::Vec3f;

type Mat4 = cgmath::Matrix4<f32>;
type CgPoint = cgmath::Point3<f32>;
type CgVec = cgmath::Vector3<f32>;
type Vector2 = cgmath::Vector2<f32>;
type Vector4 = cgmath::Vector4<f32>;

/// Manager to display the rendered image in an interactive window.
pub struct Display {
    window_dims: (u32, u32),
    event_loop: glutin::EventsLoop,
    display: glium::Display,
}

#[derive(Debug)]
pub struct CameraPose {
    pub pos: Vec3f,
    pub dir: Vec3f,
    pub up: Vec3f,
}
impl CameraPose {
    fn new(mat: &Mat4) -> CameraPose {
        let m = mat.invert().unwrap();
        let pos = m * Vector4::new(0.0, 0.0, 0.0, 1.0);
        CameraPose { pos: Vec3f::new(pos.x, pos.y, pos.z),
                     dir: -Vec3f::new(m.z.x, m.z.y, m.z.z),
                     up: Vec3f::new(m.y.x, m.y.y, m.y.z)
        }
    }
}

impl Display {
    pub fn new(w: u32, h: u32, title: &str) -> Display {
        let event_loop = glutin::EventsLoop::new();
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(w, h)
            .with_title(title);
        let context_builder = glutin::ContextBuilder::new();
        let display = glium::Display::new(window_builder, context_builder, &event_loop).unwrap();
        Display { window_dims: (w, h),
                  event_loop: event_loop,
                  display: display }
    }
    /// The function passed should render and update the image to be displayed in the window,
    /// optionally using the camera pose information passed.
    pub fn run<F>(&mut self, mut render: F) where F: FnMut(&mut RgbImage, CameraPose) {
        let mut embree_target = RgbImage::new(self.window_dims.0, self.window_dims.1);

        let mut arcball_camera = ArcballCamera::new(
            &Mat4::look_at(CgPoint::new(0.0, 0.0, -3.0), CgPoint::new(0.0, 0.0, 0.0), CgVec::new(0.0, 1.0, 0.0)),
            0.05, 1.0, [self.window_dims.0 as f32, self.window_dims.1 as f32]);

        let mut mouse_pressed = [false, false];
        let mut prev_mouse = None;
        loop {
            let mut should_quit = false;
            self.event_loop.poll_events(|e| {
                match e {
                    Event::WindowEvent { event, .. } => {
                        match event {
                            WindowEvent::Closed => should_quit = true,
                            WindowEvent::KeyboardInput { input, .. } => {
                                match input.virtual_keycode {
                                    Some(VirtualKeyCode::Escape) => should_quit = true,
                                    _ => {},
                                }
                            },
                            WindowEvent::MouseMoved { position, .. } if prev_mouse.is_none() => {
                                prev_mouse = Some(position);
                            },
                            WindowEvent::MouseMoved { position, .. } => {
                                let prev = prev_mouse.unwrap();
                                if mouse_pressed[0] {
                                    arcball_camera.rotate(Vector2::new(position.0 as f32, position.1 as f32),
                                                          Vector2::new(prev.0 as f32, prev.1 as f32));
                                } else if mouse_pressed[1] {
                                    let mouse_delta = Vector2::new((prev.0 - position.0) as f32,
                                                                   -(prev.1 - position.1) as f32);
                                    arcball_camera.pan(mouse_delta, 0.16);
                                }
                                prev_mouse = Some(position);
                            },
                            WindowEvent::MouseInput { state, button, .. } => {
                                if button == MouseButton::Left {
                                    mouse_pressed[0] = state == ElementState::Pressed;
                                } else if button == MouseButton::Right {
                                    mouse_pressed[1] = state == ElementState::Pressed;
                                }
                            },
                            WindowEvent::MouseWheel { delta, .. } => {
                                let y = match delta {
                                    MouseScrollDelta::LineDelta(_, y) => y,
                                    MouseScrollDelta::PixelDelta(_, y) => y,
                                };
                                arcball_camera.zoom(y, 0.16);
                            },

                            _ => {}
                        }
                    },
                    _ => {}
                }
            });
            if should_quit {
                return;
            }

            render(&mut embree_target, CameraPose::new(&arcball_camera.get_mat4()));
            let img = RawImage2d::from_raw_rgb_reversed(embree_target.get(..).unwrap(), self.window_dims);
            let opengl_texture = Texture2d::new(&self.display, img).unwrap();

            // Upload and blit the rendered image to display it
            let mut target = self.display.draw();
            target.clear_color(0.0, 0.0, 0.0, 0.0);
            target.clear_depth(1.0);
            opengl_texture.as_surface().fill(&target, glium::uniforms::MagnifySamplerFilter::Linear);
            target.finish().unwrap();
        }
    }
}

