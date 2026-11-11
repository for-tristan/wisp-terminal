mod app;
mod input;

use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use wisp_config::Config;
use wisp_core::emulator::Emulator;

use crate::app::{UserEvent, WispApp};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = Config::load_or_default(None).context("loading configuration")?;

    let event_loop = winit::event_loop::EventLoop::<UserEvent>::with_user_event()
        .build()
        .context("creating event loop")?;
    let proxy = event_loop.create_proxy();

    let wake_proxy = proxy.clone();
    let emulator = Emulator::new(&config, Some(Box::new(move || {
        let _ = wake_proxy.send_event(UserEvent::PtyData);
    })))
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
