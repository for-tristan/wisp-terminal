use std::io::{Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize};
use thiserror::Error;
use wisp_config::Config;

#[derive(Debug, Error)]
pub enum PtyError {
    #[error("failed to open pty: {0}")]
    Open(String),
    #[error("failed to spawn shell {shell:?}: {message}")]
    Spawn { shell: String, message: String },
    #[error("pty io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct PtyConfig {
    pub shell: Option<String>,
    pub working_dir: Option<std::path::PathBuf>,
    pub initial_cols: u16,
    pub initial_rows: u16,
    pub wake: Option<Box<dyn Fn() + Send>>,
}

impl PtyConfig {
    pub fn from_config(config: &Config) -> Self {
        Self {
            shell: config.terminal.shell.clone(),
            working_dir: None,
            initial_cols: 80,
            initial_rows: 24,
            wake: None,
        }
    }
}

pub struct PtySession {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    killer: Box<dyn ChildKiller + Send + Sync>,
    rx: Receiver<Vec<u8>>,
    exit_state: Arc<Mutex<Option<Option<u32>>>>,
    pid: Option<u32>,
    shell: String,
}

impl PtySession {
    pub fn spawn(config: &mut PtyConfig) -> Result<Self, PtyError> {
        let shell = resolve_shell(config.shell.as_deref());
        let pty_system = portable_pty::native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows: config.initial_rows,
                cols: config.initial_cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Open(e.to_string()))?;

        let mut command = CommandBuilder::new(&shell);
        if let Some(dir) = &config.working_dir {
            command.cwd(dir);
        }
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");

        let mut child = pair
            .slave
            .spawn_command(command)
            .map_err(|e| PtyError::Spawn {
                shell: shell.clone(),
                message: e.to_string(),
            })?;
        let pid = child.process_id();
        let killer = child.clone_killer();

        let exit_state = Arc::new(Mutex::new(None::<Option<u32>>));
        let exit_state_thread = Arc::clone(&exit_state);
        std::thread::Builder::new()
            .name("wisp-pty-waiter".to_string())
            .spawn(move || {
                let code = child.wait().ok().map(|status| status.exit_code() as u32);
                if let Ok(mut slot) = exit_state_thread.lock() {
                    *slot = Some(code);
                }
            })
            .map_err(|e| PtyError::Open(e.to_string()))?;

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Open(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Open(e.to_string()))?;
        let wake = config.wake.take();

        let (tx, rx) = mpsc::channel::<Vec<u8>>();
        std::thread::Builder::new()
            .name("wisp-pty-reader".to_string())
            .spawn(move || run_reader(reader, tx, wake))
            .map_err(|e| PtyError::Open(e.to_string()))?;

        Ok(Self {
            master: pair.master,
            writer,
            killer,
            rx,
            exit_state,
            pid,
            shell,
        })
    }

    pub fn shell(&self) -> &str {
        &self.shell
    }

    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()
    }

    pub fn try_recv(&self) -> Option<Vec<u8>> {
        self.rx.try_recv().ok()
    }

    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), PtyError> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Open(e.to_string()))
    }

    pub fn pid(&self) -> Option<u32> {
        self.pid
    }

    pub fn has_exited(&self) -> bool {
        self.exit_state.lock().map(|state| state.is_some()).unwrap_or(false)
    }

    pub fn exit_code(&self) -> Option<Option<u32>> {
        self.exit_state
            .lock()
            .ok()
            .and_then(|state| *state)
    }

    pub fn kill(&mut self) {
        let _ = self.killer.kill();
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = self.killer.kill();
    }
}

fn run_reader<R: Read + Send>(
    mut reader: R,
    tx: Sender<Vec<u8>>,
    wake: Option<Box<dyn Fn() + Send>>,
) {
    let mut buffer = vec![0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                if tx.send(buffer[..n].to_vec()).is_err() {
                    break;
                }
                if let Some(wake) = &wake {
                    wake();
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
}

pub fn resolve_shell(override_shell: Option<&str>) -> String {
    if let Some(shell) = override_shell {
        return shell.to_string();
    }
    if let Ok(shell) = std::env::var("SHELL") {
        if !shell.is_empty() {
            return shell;
        }
    }
    if cfg!(windows) {
        "powershell".to_string()
    } else if std::path::Path::new("/bin/bash").exists() {
        "/bin/bash".to_string()
    } else {
        "/bin/sh".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn override_wins() {
        assert_eq!(resolve_shell(Some("/bin/fish")), "/bin/fish");
    }

    #[test]
    fn falls_back_to_a_shell() {
        assert!(!resolve_shell(None).is_empty());
    }
}
