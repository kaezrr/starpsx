use crate::egui_tools::EguiRenderer;
use egui_wgpu::wgpu::SurfaceError;
use egui_wgpu::{RendererOptions, ScreenDescriptor, wgpu};
use std::sync::Arc;
use tracing::{error, trace, warn};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

// holds all the rendering stuff
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
        let selected_format = wgpu::TextureFormat::Bgra8UnormSrgb;
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
    }
}

// main application that holds the emulator and rendering state
pub struct App {
    instance: wgpu::Instance,
    state: Option<AppState>,
    label: String, // Random stuff
    value: f32,    // Random stuff
}

impl App {
    pub fn new() -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        Self {
            value: 0.0,
            label: String::new(),
            instance,
            state: None,
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
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title("StarPSX"))
            .expect("Could not create window");
        pollster::block_on(self.set_window(window));
    }

    // main emulator loop
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let mut borrowed_state = self.state.take();

        let Some(ref mut state) = borrowed_state else {
            warn!("window event called before state initialization");
            return;
        };

        state.egui_renderer.handle_input(&state.window, &event);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => state.handle_redraw(|ctx| self.show_ui(ctx)),
            WindowEvent::Resized(new_size) => state.handle_resized(new_size.width, new_size.height),
            event => trace!(?event, "ignoring window event"),
        }

        state.window.request_redraw();

        // put the borrowed state back
        self.state = borrowed_state;
    }
}

impl App {
    fn show_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::MenuBar::new().ui(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // The central panel the region left after adding TopPanel's and SidePanel's
            ui.heading("eframe template");

            ui.horizontal(|ui| {
                ui.label("Write something: ");
                ui.text_edit_singleline(&mut self.label);
            });

            ui.add(egui::Slider::new(&mut self.value, 0.0..=10.0).text("value"));
            if ui.button("Increment").clicked() {
                self.value += 1.0;
            }

            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/emilk/eframe_template/blob/main/",
                "Source code."
            ));

            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                egui::warn_if_debug_build(ui);
            });
        });
    }
}
