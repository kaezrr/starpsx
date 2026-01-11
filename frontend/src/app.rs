use crate::egui_tools::EguiRenderer;
use crate::egui_ui::show_ui;
use crate::gamepad::{convert_axis, convert_button};
use egui_wgpu::wgpu::SurfaceError;
use egui_wgpu::{RendererOptions, ScreenDescriptor, wgpu};
use std::sync::Arc;
use tracing::{error, info, trace, warn};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

// Holds all the rendering stuff
pub struct AppState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub surface: wgpu::Surface<'static>,
    pub egui_renderer: EguiRenderer,
    pub window: Arc<Window>,
}

impl AppState {
    async fn new(
        instance: &wgpu::Instance,
        surface: wgpu::Surface<'static>,
        window: Arc<Window>,
        width: u32,
        height: u32,
    ) -> Self {
        let power_pref = wgpu::PowerPreference::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power_pref,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .expect("Failed to find an appropriate adapter");

        let features = wgpu::Features::empty();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features,
                required_limits: Default::default(),
                memory_hints: Default::default(),
                trace: Default::default(),
                experimental_features: Default::default(),
            })
            .await
            .expect("Failed to create device");

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let selected_format = wgpu::TextureFormat::Bgra8Unorm;
        let swapchain_format = swapchain_capabilities
            .formats
            .iter()
            .find(|d| **d == selected_format)
            .expect("Failed to select proper surface texture format!");

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        let egui_renderer = EguiRenderer::new(
            &device,
            surface_config.format,
            RendererOptions {
                msaa_samples: 1,
                depth_stencil_format: None,
                dithering: true,
                predictable_texture_filtering: Default::default(),
            },
            &window,
        );

        Self {
            device,
            queue,
            window,
            surface,
            surface_config,
            egui_renderer,
        }
    }

    fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        } else {
            warn!(width, height, "trying to set bad window size")
        }
    }

    fn handle_redraw(&mut self, run_ui: impl FnOnce(&egui::Context)) {
        if self.window.is_minimized().unwrap_or_default() {
            warn!("not rendering while window is minimized");
            return;
        }

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: self.window.scale_factor() as f32,
        };

        let surface_texture = match self.surface.get_current_texture() {
            Err(SurfaceError::Outdated) => {
                return warn!("wgpu surface outdated");
            }
            Err(err) => {
                return error!(%err, "failed to acquire next swap chain texture");
            }
            Ok(texture) => texture,
        };

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        self.egui_renderer.begin_frame(&self.window);

        run_ui(self.egui_renderer.context());

        self.egui_renderer.end_frame_and_draw(
            &self.device,
            &self.queue,
            encoder,
            &self.window,
            &surface_view,
            screen_descriptor,
        );

        surface_texture.present();
        self.window.request_redraw();
    }
}

// Main application that holds the emulator and rendering state
pub struct App {
    instance: wgpu::Instance,
    state: Option<AppState>,
    system: starpsx_core::System,
    gamepad: gilrs::Gilrs,
}

impl App {
    pub fn new(system: starpsx_core::System) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let gamepad = gilrs::Gilrs::new().expect("Could not initialize gilrs");

        Self {
            instance,
            state: None,
            system,
            gamepad,
        }
    }

    async fn set_window(&mut self, window: Window) {
        let window = Arc::new(window);
        let initial_width = 1300;
        let initial_height = 768;

        let _ = window.request_inner_size(PhysicalSize::new(initial_width, initial_height));

        let surface = self
            .instance
            .create_surface(window.clone())
            .expect("Failed to create surface!");

        let state = AppState::new(
            &self.instance,
            surface,
            window,
            initial_width,
            initial_height,
        )
        .await;

        self.state = Some(state);
    }

    fn process_gamepad_events(&mut self) {
        let psx_gamepad = self.system.gamepad_mut();

        while let Some(gilrs::Event { event, .. }) = self.gamepad.next_event() {
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

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title("StarPSX"))
            .expect("Could not create window");
        pollster::block_on(self.set_window(window));
    }

    // This is the main emulator loop as redraw is continuosly requested
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let mut borrowed_state = self.state.take();

        let Some(ref mut state) = borrowed_state else {
            warn!("window event called before state initialization");
            return;
        };

        state.egui_renderer.handle_input(&state.window, &event);
        self.process_gamepad_events();

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => state.handle_redraw(|ctx| show_ui(ctx, &self.system)),
            WindowEvent::Resized(new_size) => state.handle_resized(new_size.width, new_size.height),
            event => trace!(?event, "ignoring window event"),
        }

        // self.system.step_frame();

        // put the borrowed state back
        self.state = borrowed_state;
    }
}
