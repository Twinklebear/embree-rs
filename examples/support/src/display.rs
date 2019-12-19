use arcball::ArcballCamera;
use cgmath::{SquareMatrix, Matrix4, Vector2, Vector3, Vector4};
use clock_ticks;
use glium::glutin::{ElementState, Event, MouseButton, MouseScrollDelta, VirtualKeyCode,
                    WindowEvent};
use glium::texture::RawImage2d;
use glium::Texture2d;
use glium::{self, glutin, Surface};
use image::RgbImage;

/// Manager to display the rendered image in an interactive window.
pub struct Display {
    window_dims: (u32, u32),
    event_loop: glutin::EventsLoop,
    display: glium::Display,
}

#[derive(Debug)]
pub struct CameraPose {
    pub pos: Vector3<f32>,
    pub dir: Vector3<f32>,
    pub up: Vector3<f32>,
}
impl CameraPose {
    fn new(pos: Vector3<f32>, dir: Vector3<f32>, up: Vector3<f32>) -> CameraPose {
        CameraPose {
            pos,
            dir,
            up,
        }
    }
}

impl Display {
    pub fn new(w: u32, h: u32, title: &str) -> Display {
        let event_loop = glutin::EventsLoop::new();
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(glutin::dpi::LogicalSize::new(w.into(), h.into()))
            .with_title(title);
        let context_builder = glutin::ContextBuilder::new();
        let display = glium::Display::new(window_builder, context_builder, &event_loop).unwrap();
        Display {
            window_dims: (w, h),
            event_loop: event_loop,
            display: display,
        }
    }
    /// The function passed should render and update the image to be displayed in the window,
    /// optionally using the camera pose information passed.
    pub fn run<F>(&mut self, mut render: F)
    where
        F: FnMut(&mut RgbImage, CameraPose, f32),
    {
        let mut embree_target = RgbImage::new(self.window_dims.0, self.window_dims.1);

        let mut arcball_camera = ArcballCamera::new(Vector3::new(0.0, 0.0, 0.0),
            1.0,
            [self.window_dims.0 as f32, self.window_dims.1 as f32],
        );
        arcball_camera.zoom(-50.0, 0.16);
        arcball_camera.rotate(Vector2::new(self.window_dims.0 as f32 / 2.0,
                                           self.window_dims.1 as f32 / 4.0),
                                Vector2::new(self.window_dims.0 as f32 / 2.0,
                                             self.window_dims.1 as f32 / 3.0));

        let mut mouse_pressed = [false, false];
        let mut prev_mouse = None;
        let t_start = clock_ticks::precise_time_s();
        loop {
            let mut should_quit = false;
            self.event_loop.poll_events(|e| match e {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => should_quit = true,
                    WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                        Some(VirtualKeyCode::Escape) => should_quit = true,
                        _ => {}
                    },
                    WindowEvent::CursorMoved { position, .. } if prev_mouse.is_none() => {
                        prev_mouse = Some(position);
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        let prev = prev_mouse.unwrap();
                        if mouse_pressed[0] {
                            arcball_camera.rotate(
                                Vector2::new(position.x as f32, prev.y as f32),
                                Vector2::new(prev.x as f32, position.y as f32)
                            );
                        } else if mouse_pressed[1] {
                            let mouse_delta = Vector2::new(
                                (prev.x - position.x) as f32,
                                (position.y - prev.y) as f32
                            );
                            arcball_camera.pan(mouse_delta, 0.16);
                        }
                        prev_mouse = Some(position);
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if button == MouseButton::Left {
                            mouse_pressed[0] = state == ElementState::Pressed;
                        } else if button == MouseButton::Right {
                            mouse_pressed[1] = state == ElementState::Pressed;
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        let y = match delta {
                            MouseScrollDelta::LineDelta(_, y) => y,
                            MouseScrollDelta::PixelDelta(lpos) => lpos.y as f32,
                        };
                        arcball_camera.zoom(y, 0.16);
                    }
                    _ => {}
                },
                _ => {}
            });
            if should_quit {
                return;
            }

            let cam_pose = CameraPose::new(arcball_camera.eye_pos(), arcball_camera.eye_dir(),
                                           arcball_camera.up_dir());
            render(
                &mut embree_target,
                cam_pose,
                (clock_ticks::precise_time_s() - t_start) as f32,
            );
            let img =
                RawImage2d::from_raw_rgb_reversed(embree_target.get(..).unwrap(), self.window_dims);
            let opengl_texture = Texture2d::new(&self.display, img).unwrap();

            // Upload and blit the rendered image to display it
            let target = self.display.draw();
            opengl_texture
                .as_surface()
                .fill(&target, glium::uniforms::MagnifySamplerFilter::Linear);
            target.finish().unwrap();
        }
    }
}
