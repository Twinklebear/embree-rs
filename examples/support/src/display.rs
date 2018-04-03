use glium::{self, glutin, Surface};
use glium::glutin::{Event, WindowEvent, VirtualKeyCode, ElementState, MouseScrollDelta, MouseButton};
use glium::texture::RawImage2d;
use glium::Texture2d;
use image::RgbImage;
use arcball::ArcballCamera;
use cgmath::SquareMatrix;
use clock_ticks;

use ::{Mat4, CgPoint, CgVec, Vector2, Vector3, Vector4};

/// Manager to display the rendered image in an interactive window.
pub struct Display {
    window_dims: (u32, u32),
    event_loop: glutin::EventsLoop,
    display: glium::Display,
}

#[derive(Debug)]
pub struct CameraPose {
    pub pos: Vector3,
    pub dir: Vector3,
    pub up: Vector3,
}
impl CameraPose {
    fn new(mat: &Mat4) -> CameraPose {
        let m = mat.invert().unwrap();
        let pos = m * Vector4::new(0.0, 0.0, 0.0, 1.0);
        CameraPose { pos: Vector3::new(pos.x, pos.y, pos.z),
                     dir: -Vector3::new(m.z.x, m.z.y, m.z.z),
                     up: Vector3::new(m.y.x, m.y.y, m.y.z)
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
    pub fn run<F>(&mut self, mut render: F) where F: FnMut(&mut RgbImage, CameraPose, f32) {
        let mut embree_target = RgbImage::new(self.window_dims.0, self.window_dims.1);

        let mut arcball_camera = ArcballCamera::new(
            &Mat4::look_at(CgPoint::new(0.0, 1.0, -6.0), CgPoint::new(0.0, 0.0, 0.0), CgVec::new(0.0, 1.0, 0.0)),
            0.05, 1.0, [self.window_dims.0 as f32, self.window_dims.1 as f32]);

        let mut mouse_pressed = [false, false];
        let mut prev_mouse = None;
        let t_start = clock_ticks::precise_time_s();
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
                            WindowEvent::CursorMoved { position, .. } if prev_mouse.is_none() => {
                                prev_mouse = Some(position);
                            },
                            WindowEvent::CursorMoved { position, .. } => {
                                let prev = prev_mouse.unwrap();
                                if mouse_pressed[0] {
                                    arcball_camera.rotate(Vector2::new(position.0 as f32, position.1 as f32),
                                                          Vector2::new(prev.0 as f32, prev.1 as f32));
                                } else if mouse_pressed[1] {
                                    let mouse_delta = Vector2::new((prev.0 - position.0) as f32,
                                                                   (prev.1 - position.1) as f32);
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

            render(&mut embree_target, CameraPose::new(&arcball_camera.get_mat4()),
                   (clock_ticks::precise_time_s() - t_start) as f32);
            let img = RawImage2d::from_raw_rgb_reversed(embree_target.get(..).unwrap(), self.window_dims);
            let opengl_texture = Texture2d::new(&self.display, img).unwrap();

            // Upload and blit the rendered image to display it
            let target = self.display.draw();
            opengl_texture.as_surface().fill(&target, glium::uniforms::MagnifySamplerFilter::Linear);
            target.finish().unwrap();
        }
    }
}

