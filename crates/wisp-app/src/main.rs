mod app;
mod cli;
mod input;

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::Parser as _;
use wisp_config::Config;
use wisp_core::emulator::Emulator;

use crate::app::{UserEvent, WispApp};
use crate::cli::Cli;

fn main() -> Result<()> {
    std::panic::set_hook(Box::new(|info| {
        log::error!("wisp panicked: {info}");
    }));

    let cli = Cli::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(cli.log_level()))
        .init();

    if cli.print_config_path {
        match Config::default_path() {
            Some(path) => println!("{}", path.display()),
            None => println!("no default config location on this platform"),
        }
        return Ok(());
    }

    let config = Config::load_or_default(cli.config.as_deref()).context("loading configuration")?;

    let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event()
        .build()
        .context("creating event loop")?;
    let proxy = event_loop.create_proxy();

    let wake_proxy = proxy.clone();
    let working_dir = cli.working_dir.clone();
    let emulator = Emulator::new(
        &config,
        working_dir,
        Some(Box::new(move || {
            let _ = wake_proxy.send_event(UserEvent::PtyData);
        })),
    )
    .context("starting shell session")?;

    let mut app = WispApp::new(config.clone(), emulator);

    if config.cursor.blink {
        let blink_proxy = proxy.clone();
        let interval = config.cursor.blink_interval_ms.clamp(100, 2000);
        std::thread::Builder::new()
            .name("wisp-blink".to_string())
            .spawn(move || loop {
                std::thread::sleep(Duration::from_millis(interval));
                let _ = blink_proxy.send_event(UserEvent::BlinkTick);
            })
            .context("spawning blink timer")?;
    }

    event_loop
        .run_app(&mut app)
        .map_err(|error| anyhow!("event loop: {error}"))
}
