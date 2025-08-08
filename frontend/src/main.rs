mod app;
use winit::error::EventLoopError;

fn main() -> Result<(), EventLoopError> {
    let event_loop = winit::event_loop::EventLoop::new()?;
    let config = starpsx_core::Config::build().unwrap_or_else(|err| {
        eprintln!("Error parsing arguments: {err}");
        std::process::exit(1);
    });

    event_loop.run_app(&mut app::App {
        state: None,
        config: Some(config),
    })
}
