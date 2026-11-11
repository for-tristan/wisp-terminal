use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{Ime, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};
use wisp_config::Config;
use wisp_core::emulator::{AppEvent, Emulator};
use wisp_render::renderer::Renderer;

use crate::input;

#[derive(Debug)]
pub enum UserEvent {
    PtyData,
}

pub struct WispApp {
    pub config: Config,
    pub emulator: Emulator,
    pub window: Option<Arc<Window>>,
    pub renderer: Option<Renderer>,
    pub modifiers: ModifiersState,
    pub focused: bool,
    pub exited: bool,
    pub window_size: Option<PhysicalSize<u32>>,
}

impl WispApp {
    pub fn new(config: Config, emulator: Emulator) -> Self {
        Self {
            config,
            emulator,
            window: None,
            renderer: None,
            modifiers: ModifiersState::default(),
            focused: false,
            exited: false,
            window_size: None,
        }
    }

    fn background_rgb(&self) -> [u8; 3] {
        self.config
            .theme()
            .palette
            .color("background")
            .map(|color| color.to_tuple())
            .map(|(r, g, b)| [r, g, b])
            .unwrap_or([0x1A, 0x22, 0x2C])
    }

    fn request_redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn pump(&mut self) -> bool {
        let result = self.emulator.pump();
        for event in result.events {
            match event {
                AppEvent::TitleChanged(title) => {
                    if let Some(window) = &self.window {
                        window.set_title(&title);
                    }
                }
                AppEvent::ChildExited(code) => {
                    self.emulator.notify_exit(code);
                    self.exited = true;
                    self.request_redraw();
                }
                AppEvent::Bell | AppEvent::ClipboardStore(_) | AppEvent::ClipboardLoadRequest(_) => {}
            }
        }
        result.damaged
    }
}

impl ApplicationHandler<UserEvent> for WispApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let title = self.config.window.title.clone();
        let attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(1024.0, 640.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(
                self.config.window.min_width,
                self.config.window.min_height,
            ))
            .with_transparent(self.config.window.opacity < 1.0);
        let window = event_loop
            .create_window(attributes)
            .expect("failed to create window");
        self.window = Some(Arc::new(window));

        let background = self.background_rgb();
        match Renderer::new(self.window.clone().expect("window"), background) {
            Ok(renderer) => self.renderer = Some(renderer),
            Err(error) => log::error!("renderer init failed: {error}"),
        }

        self.request_redraw();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::PtyData => {
                if self.pump() {
                    self.request_redraw();
                }
            }
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, window_id: WindowId, event: WindowEvent) {
        let is_main_window = self
            .window
            .as_ref()
            .is_some_and(|window| window.id() == window_id);
        if !is_main_window {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                self.window_size = Some(size);
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                self.request_redraw();
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.request_redraw();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let Some(bytes) = input::encode_key(&event, self.modifiers) {
                    let _ = self.emulator.input(&bytes);
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                let _ = self.emulator.paste(&text);
            }
            WindowEvent::Ime(_) => {}
            WindowEvent::MouseInput { state, button, .. } => {
                let _ = (state, button);
            }
            WindowEvent::CursorMoved { position, .. } => {
                let _ = position;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, lines) => lines as i32,
                    MouseScrollDelta::PixelDelta(position) => {
                        let (_, cell_height) = self.emulator.cell_size();
                        let pixels = position.y as f32;
                        (pixels / cell_height.max(1) as f32) as i32
                    }
                };
                if lines != 0 {
                    self.emulator.scroll(lines);
                    self.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                let damaged = self.pump();
                if let Some(renderer) = &mut self.renderer {
                    if let Err(error) = renderer.render() {
                        log::error!("frame render failed: {error}");
                    }
                }
                let _ = damaged;
                if self.exited {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}
}
