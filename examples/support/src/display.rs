use glium::{self, glutin, Surface};
use glium::texture::RawImage2d;
use glium::Texture2d;
use image::RgbImage;

/// Manager to display the rendered image in an interactive window.
pub struct Display {
    window_dims: (u32, u32),
    event_loop: glutin::EventsLoop,
    display: glium::Display,
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
    /// The function passed should render and return the image to be displayed
    /// in the window, e.g. using Embree. The image returned should be RGB8 format
    /// TODO: Should I have this work on an image::DynamicImage instead?
    pub fn run<F>(&mut self, mut render: F) where F: FnMut(&mut RgbImage) {
        let mut embree_target = RgbImage::new(self.window_dims.0, self.window_dims.1);
        loop {
            let mut should_quit = false;
            self.event_loop.poll_events(|e| {
                use glium::glutin::{Event, WindowEvent, VirtualKeyCode};
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
                            _ => {}
                        }
                    },
                    _ => {}
                }
            });
            if should_quit {
                return;
            }

            render(&mut embree_target);
            let img = RawImage2d::from_raw_rgb_reversed(embree_target.get(..).unwrap(), self.window_dims);
            let opengl_texture = Texture2d::new(&self.display, img).unwrap();

            // Upload and blit the rendered image to display it
            let target = self.display.draw();
            opengl_texture.as_surface().fill(&target, glium::uniforms::MagnifySamplerFilter::Linear);
            target.finish().unwrap();
        }
    }
}

