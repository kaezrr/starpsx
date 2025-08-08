use pixels::{Pixels, SurfaceTexture};
use std::sync::Arc;
use std::time::Instant;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::event_loop::ControlFlow;
use winit::window::Window;
use winit::{application::ApplicationHandler, event::WindowEvent};

const WINDOW_SIZE: LogicalSize<u32> = LogicalSize::new(640, 480);

struct AppState<'a> {
    window: Arc<Window>,
    pixels: Pixels<'a>,
    last_frame: Instant,
}

#[derive(Default)]
struct App<'a> {
    state: Option<AppState<'a>>,
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_some() {
            eprintln!("Resume called after window initialization.");
            return;
        }
        let win_attr = Window::default_attributes()
            .with_title("StarPSX")
            .with_inner_size(WINDOW_SIZE);
        let window = Arc::new(event_loop.create_window(win_attr).unwrap());
        let surface_texture =
            SurfaceTexture::new(WINDOW_SIZE.width, WINDOW_SIZE.height, window.clone());
        let mut pixels =
            Pixels::new(WINDOW_SIZE.width, WINDOW_SIZE.height, surface_texture).unwrap();
        pixels.enable_vsync(false);

        self.state = Some(AppState {
            pixels,
            window,
            last_frame: Instant::now(),
        })
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            eprintln!("RedrawRequested fired before Resumed or after Suspended");
            return;
        };

        match event {
            WindowEvent::RedrawRequested => {
                draw(state.pixels.frame_mut());
                if let Err(err) = state.pixels.render() {
                    eprintln!("Pixels could not render: {err}");
                    event_loop.exit();
                    return;
                }

                let now = Instant::now();
                let delta = now - state.last_frame;
                state.last_frame = now;
                let fps = 1.0 / delta.as_secs_f64();
                eprintln!("{fps:.4}");
                state.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            event => eprintln!("Ignoring window event: {event:?}"),
        }
    }
}

fn draw(buffer: &mut [u8]) {
    let width = 640;
    let height = 480;

    assert_eq!(buffer.len(), width * height * 4);

    for y in 0..height {
        for x in 0..width {
            let i = (y * width + x) * 4;

            let red = ((x as f32 / width as f32) * 255.0) as u8;
            let green = ((y as f32 / height as f32) * 255.0) as u8;
            let blue = 0;
            let alpha = 255;

            buffer[i] = red;
            buffer[i + 1] = green;
            buffer[i + 2] = blue;
            buffer[i + 3] = alpha;
        }
    }
}

fn main() -> Result<(), EventLoopError> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::default())
}
