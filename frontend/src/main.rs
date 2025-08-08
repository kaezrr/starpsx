use softbuffer::Surface;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Instant;
use winit::dpi::LogicalSize;
use winit::error::EventLoopError;
use winit::event_loop::ControlFlow;
use winit::window::Window;
use winit::{application::ApplicationHandler, event::WindowEvent};

const WINDOW_SIZE: LogicalSize<u32> = LogicalSize::new(640, 480);

struct AppState {
    window: Rc<Window>,
    draw_surface: Surface<Rc<Window>, Rc<Window>>,
    last_frame: Instant,
}

#[derive(Default)]
struct App {
    state: Option<AppState>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_some() {
            eprintln!("Resume called after window initialization.");
            return;
        }
        let win_attr = Window::default_attributes()
            .with_title("StarPSX")
            .with_inner_size(WINDOW_SIZE);
        let window = Rc::new(event_loop.create_window(win_attr).unwrap());
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let draw_surface = softbuffer::Surface::new(&context, window.clone()).unwrap();

        self.state = Some(AppState {
            window,
            draw_surface,
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
                state
                    .draw_surface
                    .resize(
                        NonZeroU32::new(WINDOW_SIZE.width).unwrap(),
                        NonZeroU32::new(WINDOW_SIZE.height).unwrap(),
                    )
                    .unwrap();
                let mut buffer = state.draw_surface.buffer_mut().unwrap();
                draw(&mut buffer);
                buffer.present().unwrap();

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

fn draw(buffer: &mut [u32]) {
    let width = 640;
    let height = 480;

    assert_eq!(buffer.len(), width * height);

    // Precompute scaling factors
    let inv_width = 255.0 / (width - 1) as f32;
    let inv_height = 255.0 / (height - 1) as f32;

    let red_lut: Vec<u32> = (0..width)
        .map(|x| ((x as f32 * inv_width) as u32) << 16)
        .collect();
    let blue_lut: Vec<u32> = (0..height)
        .map(|y| (y as f32 * inv_height) as u32)
        .collect();

    for (y, bval) in blue_lut.iter().enumerate().take(height) {
        let blue = bval;
        let row_start = y * width;

        for (x, rval) in red_lut.iter().enumerate().take(width) {
            let red = rval;
            buffer[row_start + x] = red | blue;
        }
    }
}

fn main() -> Result<(), EventLoopError> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::default())
}
