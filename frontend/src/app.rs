use softbuffer::Surface;
use starpsx_core::gpu::renderer::{CANVAS_HEIGHT, CANVAS_WIDTH};
use starpsx_core::{Config, StarPSX};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Instant;
use winit::{application::ApplicationHandler, event::WindowEvent};
use winit::{dpi::LogicalSize, window::Window};

const WINDOW_SIZE: LogicalSize<u32> = LogicalSize::new(CANVAS_WIDTH as u32, CANVAS_HEIGHT as u32);

pub struct AppState {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    last_frame: Instant,
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
            let draw_surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
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
                surface: draw_surface,
                last_frame: Instant::now(),
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

        match event {
            WindowEvent::RedrawRequested => {
                state.draw_to_screen();
                state.show_fps();
                state.window.request_redraw();
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            event => eprintln!("Ignoring window event: {event:?}"),
        }
    }
}

impl AppState {
    fn draw_to_screen(&mut self) {
        self.surface
            .resize(
                NonZeroU32::new(WINDOW_SIZE.width).unwrap(),
                NonZeroU32::new(WINDOW_SIZE.height).unwrap(),
            )
            .unwrap();

        let mut buffer = self.surface.buffer_mut().unwrap();
        buffer.copy_from_slice(self.psx.pixel_buffer());
        buffer.present().unwrap();
    }

    fn show_fps(&mut self) {
        let now = Instant::now();
        let delta = now - self.last_frame;
        self.last_frame = now;
        let fps = 1.0 / delta.as_secs_f64();
        eprintln!("{fps:.4}");
    }
}
