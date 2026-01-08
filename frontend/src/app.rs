use gilrs::{Event, Gilrs};
use softbuffer::Surface;
use starpsx_core::{Config, System, TARGET_FPS, gamepad};
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::{Duration, Instant};
use tracing::{error, info, trace, warn};
use winit::{application::ApplicationHandler, event::WindowEvent};
use winit::{dpi::LogicalSize, window::Window};

const WINDOW_SIZE: LogicalSize<u32> = if cfg!(feature = "full-vram") {
    LogicalSize::new(1600, 800)
} else {
    LogicalSize::new(960, 720)
};

const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);

pub struct AppState {
    window: Rc<Window>,
    surface: Surface<Rc<Window>, Rc<Window>>,
    system: System,
    gamepad: Gilrs,
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
            let gamepad = Gilrs::new().unwrap();
            let system = match System::build(config) {
                Ok(psx) => psx,
                Err(err) => {
                    error!(%err, "could not initialize emulator");
                    std::process::exit(1);
                }
            };

            self.state = Some(AppState {
                system,
                window,
                surface,
                gamepad,
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
            warn!("RedrawRequested fired before Resumed or after Suspended");
            return;
        };

        let frame_start = Instant::now();

        state.process_input_events();
        state.system.step_frame();

        state.draw_to_screen();
        state.window.request_redraw();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            event => trace!(?event, "ignoring window event"),
        }

        // Thread sleeping locks the framerate here
        let elapsed = frame_start.elapsed();
        let actual_fps = 1.0 / elapsed.as_secs_f64();
        state
            .window
            .set_title(&format!("StarPSX - {actual_fps:.2} FPS"));

        if let Some(remaining) = FRAME_TIME.checked_sub(elapsed) {
            std::thread::sleep(remaining);
        } else {
            warn!("frame took too long to render");
        }
    }
}

impl AppState {
    fn draw_to_screen(&mut self) {
        let (width, height) = self.system.get_resolution();
        self.surface
            .resize(
                NonZeroU32::new(width).unwrap(),
                NonZeroU32::new(height).unwrap(),
            )
            .unwrap();

        let mut buffer = self.surface.buffer_mut().unwrap();
        buffer.copy_from_slice(self.system.frame_buffer());
        buffer.present().unwrap();
    }

    fn process_input_events(&mut self) {
        let psx_gamepad = self.system.gamepad_mut();

        while let Some(Event { event, .. }) = self.gamepad.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(gilrs::Button::Mode, _) => {}
                gilrs::EventType::ButtonReleased(gilrs::Button::Mode, _) => {
                    psx_gamepad.toggle_analog_mode()
                }

                gilrs::EventType::ButtonPressed(button, _) => {
                    psx_gamepad.set_button_state(convert_button(button), true)
                }

                gilrs::EventType::ButtonReleased(button, _) => {
                    psx_gamepad.set_button_state(convert_button(button), false)
                }

                gilrs::EventType::Connected => {
                    info!("gamepad connected")
                }

                gilrs::EventType::Disconnected => {
                    info!("gamepad disconnected")
                }

                gilrs::EventType::AxisChanged(axis, value, _) => {
                    let (converted_axis, new_value) = convert_axis(axis, value);
                    psx_gamepad.set_stick_axis(converted_axis, new_value);
                }

                _ => trace!(?event, "gamepad event ignored"),
            }
        }
    }
}

fn convert_axis(axis: gilrs::Axis, value: f32) -> (gamepad::StickAxis, u8) {
    // Y axis is flipped between gilrs and console
    let v = match axis {
        gilrs::Axis::LeftStickY | gilrs::Axis::RightStickY => -value,
        _ => value,
    };

    let byte = ((v + 1.0) * 127.5).round().clamp(0.0, 255.0) as u8;

    let mapped = match axis {
        gilrs::Axis::RightStickX => gamepad::StickAxis::RightX,
        gilrs::Axis::RightStickY => gamepad::StickAxis::RightY,

        gilrs::Axis::LeftStickX => gamepad::StickAxis::LeftX,
        gilrs::Axis::LeftStickY => gamepad::StickAxis::LeftY,

        _ => unimplemented!("unmapped gamepad axis"),
    };

    (mapped, byte)
}

fn convert_button(gilrs_button: gilrs::Button) -> gamepad::Button {
    match gilrs_button {
        // Face buttons
        gilrs::Button::South => gamepad::Button::Cross,
        gilrs::Button::East => gamepad::Button::Circle,
        gilrs::Button::North => gamepad::Button::Triangle,
        gilrs::Button::West => gamepad::Button::Square,

        // Shoulders / Triggers
        gilrs::Button::LeftTrigger => gamepad::Button::L1,
        gilrs::Button::LeftTrigger2 => gamepad::Button::L2,
        gilrs::Button::RightTrigger => gamepad::Button::R1,
        gilrs::Button::RightTrigger2 => gamepad::Button::R2,

        // Menu
        gilrs::Button::Select => gamepad::Button::Select,
        gilrs::Button::Start => gamepad::Button::Start,

        // Thumbsticks
        gilrs::Button::LeftThumb => gamepad::Button::L3,
        gilrs::Button::RightThumb => gamepad::Button::R3,

        // D-Pad
        gilrs::Button::DPadUp => gamepad::Button::Up,
        gilrs::Button::DPadDown => gamepad::Button::Down,
        gilrs::Button::DPadLeft => gamepad::Button::Left,
        gilrs::Button::DPadRight => gamepad::Button::Right,

        _ => unimplemented!("unmapped gamepad button"),
    }
}
