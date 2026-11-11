use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::ActiveEventLoop;
use winit::keyboard::ModifiersState;
use winit::window::{Window, WindowId};
use wisp_config::Config;
use wisp_core::emulator::{AppEvent, Emulator};
use wisp_core::Side;
use wisp_render::renderer::Renderer;

use crate::input::{self, EncodedInput, MouseState};

#[derive(Debug)]
pub enum UserEvent {
    PtyData,
    BlinkTick,
}

pub struct WispApp {
    pub config: Config,
    pub emulator: Emulator,
    pub window: Option<Arc<Window>>,
    pub renderer: Option<Renderer>,
    pub clipboard: Option<arboard::Clipboard>,
    pub mouse: MouseState,
    pub modifiers: ModifiersState,
    pub focused: bool,
    pub exited: bool,
    pub blink_on: bool,
    pub needs_redraw: bool,
    pub window_size: Option<PhysicalSize<u32>>,
}

impl WispApp {
    pub fn new(config: Config, emulator: Emulator) -> Self {
        Self {
            config,
            emulator,
            window: None,
            renderer: None,
            clipboard: None,
            mouse: MouseState::new(),
            modifiers: ModifiersState::default(),
            focused: false,
            exited: false,
            blink_on: true,
            needs_redraw: true,
            window_size: None,
        }
    }

    fn sync_grid(&mut self) {
        let Some(renderer) = &self.renderer else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };
        let size = window.inner_size();
        let (cell_width, cell_height) = renderer.metrics();
        let padding = self.config.window.padding;
        let usable_width = size.width.saturating_sub(2 * padding.x as u32);
        let usable_height = size.height.saturating_sub(2 * padding.y as u32);
        let cols = (usable_width as f32 / cell_width).floor().max(2.0) as usize;
        let rows = (usable_height as f32 / cell_height).floor().max(2.0) as usize;
        if self.emulator.resize(cols, rows).is_ok() {
            let renderer = self.renderer.as_mut().expect("renderer checked above");
            renderer.set_origin(padding.x as f32, padding.y as f32);
            self.mouse.origin = (padding.x as f32, padding.y as f32);
            let (cell_width, cell_height) = renderer.metrics();
            self.mouse.cell_size = (cell_width, cell_height);
            self.mouse.grid = (cols, rows);
        }
    }

    fn clipboard(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
        self.clipboard.as_mut()
    }

    fn copy_selection(&mut self) {
        if let Some(text) = self.emulator.selected_text() {
            if let Some(clipboard) = self.clipboard() {
                if let Err(error) = clipboard.set_text(text) {
                    log::warn!("clipboard set failed: {error}");
                }
            }
        }
    }

    fn paste_clipboard(&mut self) {
        let text = self
            .clipboard()
            .and_then(|clipboard| clipboard.get_text().ok());
        if let Some(text) = text {
            let _ = self.emulator.paste(&text);
        }
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
                    self.needs_redraw = true;
                    self.request_redraw();
                }
                AppEvent::Bell => {}
                AppEvent::ClipboardStore(text) => {
                    if let Some(clipboard) = self.clipboard() {
                        let _ = clipboard.set_text(text);
                    }
                }
                AppEvent::ClipboardLoadRequest(formatter) => {
                    if let Some(text) = self.clipboard().and_then(|c| c.get_text().ok()) {
                        let _ = self.emulator.write(formatter(&text).as_bytes());
                    }
                }
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
        window.set_cursor(winit::window::Cursor::Icon(winit::window::CursorIcon::Text));
        self.window = Some(Arc::new(window));

        let theme = self.config.theme();
        let font_config = self.config.font.clone();
        let opacity = self.config.window.opacity;
        match Renderer::new(
            self.window.clone().expect("window"),
            &theme,
            &font_config,
            opacity,
        ) {
            Ok(renderer) => {
                let (cell_width, cell_height) = renderer.metrics();
                self.emulator.set_cell_size(cell_width as u16, cell_height as u16);
                self.renderer = Some(renderer);
                self.sync_grid();
            }
            Err(error) => log::error!("renderer init failed: {error}"),
        }

        self.request_redraw();
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::PtyData => {
                if self.pump() {
                    self.needs_redraw = true;
                    self.request_redraw();
                }
            }
            UserEvent::BlinkTick => {
                self.blink_on = !self.blink_on;
                self.needs_redraw = true;
                self.request_redraw();
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
                self.sync_grid();
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
                if self.exited {
                    event_loop.exit();
                    return;
                }
                if let Some(encoded) = input::encode_key(&event, self.modifiers) {
                    match encoded {
                        EncodedInput::Bytes(bytes) => {
                            self.emulator.clear_selection();
                            self.needs_redraw = true;
                            let _ = self.emulator.input(&bytes);
                        }
                        EncodedInput::Copy => self.copy_selection(),
                        EncodedInput::Paste => self.paste_clipboard(),
                    }
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.emulator.clear_selection();
                let _ = self.emulator.paste(&text);
            }
            WindowEvent::Ime(_) => {}
            WindowEvent::MouseInput { state, button, .. } => {
                let position = self.mouse.position;
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        let now = std::time::Instant::now();
                        let multi = self
                            .mouse
                            .last_click
                            .is_some_and(|last| now.duration_since(last).as_millis() < 400);
                        self.mouse.click_count = if multi {
                            self.mouse.click_count + 1
                        } else {
                            1
                        };
                        self.mouse.last_click = Some(now);
                        self.mouse.left_down = true;
                        if let Some(point) =
                            position.and_then(|position| self.mouse.viewport_point(position))
                        {
                            if let Some(ty) = input::selection_type_for_click(self.mouse.click_count)
                            {
                                self.emulator.begin_selection(point, ty);
                                self.needs_redraw = true;
                            }
                        }
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.mouse.left_down = false;
                    }
                    (MouseButton::Middle, ElementState::Pressed) => {
                        self.paste_clipboard();
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        if let Some(point) =
                            position.and_then(|position| self.mouse.viewport_point(position))
                        {
                            let side = side_for_position(position, &self.mouse);
                            self.emulator.update_selection(point, side);
                            self.needs_redraw = true;
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse.position = Some(position);
                if self.mouse.left_down {
                    if let Some(point) = self.mouse.viewport_point(position) {
                        let side = side_for_position(Some(position), &self.mouse);
                        self.emulator.update_selection(point, side);
                        self.needs_redraw = true;
                    }
                }
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
                    let _ = self.emulator.wheel(lines);
                    self.needs_redraw = true;
                    self.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if self.pump() {
                    self.needs_redraw = true;
                }
                if self.needs_redraw {
                    if let Some(renderer) = &mut self.renderer {
                        let snapshot = self.emulator.snapshot();
                        let cursor_visible = self.blink_on || !self.config.cursor.blink;
                        if let Err(error) = renderer.render(&snapshot, cursor_visible, self.focused) {
                            log::error!("frame render failed: {error}");
                        }
                    }
                    self.needs_redraw = false;
                }
                if self.exited {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {}
}

fn side_for_position(position: Option<PhysicalPosition<f64>>, mouse: &MouseState) -> Side {
    let Some(position) = position else {
        return Side::Right;
    };
    let (ox, _) = mouse.origin;
    let (cw, _) = mouse.cell_size;
    if cw <= 0.0 {
        return Side::Right;
    }
    let within = (position.x - ox as f64) % cw as f64;
    if within < cw as f64 * 0.5 {
        Side::Left
    } else {
        Side::Right
    }
}
