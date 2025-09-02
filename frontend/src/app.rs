use softbuffer::Surface;
use starpsx_core::{Config, StarPSX, TARGET_FPS};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};
use winit::{application::ApplicationHandler, event::WindowEvent};
use winit::{dpi::LogicalSize, window::Window};

const WINDOW_SIZE: LogicalSize<u32> = LogicalSize::new(960, 720);
const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);

pub struct AppState {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    psx: StarPSX,
}

#[derive(Default)]
pub struct App {
    pub state: Option<AppState>,
    pub config: Option<Config>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(config) = self.config.take() {
            let win_attr = Window::default_attributes()
                .with_title("StarPSX")
                .with_inner_size(WINDOW_SIZE);
            let window = Rc::new(event_loop.create_window(win_attr).unwrap());
            let context = softbuffer::Context::new(window.clone()).unwrap();
            let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
            let psx = match StarPSX::build(config) {
                Ok(psx) => psx,
                Err(err) => {
                    eprintln!("Error building emulator: {err}");
                    return event_loop.exit();
                }
            };

            self.state = Some(AppState {
                psx,
                window,
                surface,
            })
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if event_loop.exiting() {
            return;
        }

        let Some(state) = self.state.as_mut() else {
            eprintln!("RedrawRequested fired before Resumed or after Suspended");
            return;
        };

        let frame_start = Instant::now();
        state.psx.step_frame();

        match event {
            WindowEvent::RedrawRequested => {
                state.draw_to_screen();
                state.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            event => eprintln!("Ignoring window event: {event:?}"),
        }

        // Thread sleeping locks the framerate here
        let elapsed = frame_start.elapsed();
        // let actual_fps = 1.0 / elapsed.as_secs_f64();
        // println!("FPS: {actual_fps:.2}");

        if let Some(remaining) = FRAME_TIME.checked_sub(elapsed) {
            std::thread::sleep(remaining);
        } else {
            eprintln!("Frame took too long to render");
        }
    }
}

impl AppState {
    fn draw_to_screen(&mut self) {
        let (width, height) = self.psx.get_resolution();
        self.surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .unwrap();

        let mut buffer = self.surface.buffer_mut().unwrap();
        buffer.copy_from_slice(self.psx.frame_buffer());
        buffer.present().unwrap();
    }
}
