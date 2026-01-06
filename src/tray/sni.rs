use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::error::TrayError;

/// Tray icon states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TrayState {
    Idle = 0,
    Recording = 1,
    Processing = 2,
    Error = 3,
}

impl From<u8> for TrayState {
    fn from(v: u8) -> Self {
        match v {
            1 => TrayState::Recording,
            2 => TrayState::Processing,
            3 => TrayState::Error,
            _ => TrayState::Idle,
        }
    }
}

impl TrayState {
    fn icon_name(&self) -> &'static str {
        match self {
            TrayState::Idle => "audio-input-microphone",
            TrayState::Recording => "media-record",
            TrayState::Processing => "system-run",
            TrayState::Error => "dialog-error",
        }
    }

    fn tooltip(&self) -> &'static str {
        match self {
            TrayState::Idle => "SuperWhisper - Ready",
            TrayState::Recording => "SuperWhisper - Recording...",
            TrayState::Processing => "SuperWhisper - Processing...",
            TrayState::Error => "SuperWhisper - Error",
        }
    }

    fn status(&self) -> ksni::Status {
        match self {
            TrayState::Idle => ksni::Status::Passive,
            TrayState::Recording => ksni::Status::Active,
            TrayState::Processing => ksni::Status::Active,
            TrayState::Error => ksni::Status::NeedsAttention,
        }
    }
}

/// Handle to control the tray from outside
#[derive(Clone)]
pub struct TrayHandle {
    state: Arc<AtomicU8>,
    handle: ksni::Handle<SuperWhisperTray>,
}

impl TrayHandle {
    /// Update the tray state
    pub fn set_state(&self, state: TrayState) {
        let old = self.state.swap(state as u8, Ordering::SeqCst);
        if old != state as u8 {
            debug!("Tray state: {:?} -> {:?}", TrayState::from(old), state);
            self.handle.update(|_| {});
        }
    }

    /// Get current state
    pub fn state(&self) -> TrayState {
        self.state.load(Ordering::SeqCst).into()
    }
}

/// The actual tray implementation
struct SuperWhisperTray {
    state: Arc<AtomicU8>,
    socket_path: String,
}

impl ksni::Tray for SuperWhisperTray {
    fn id(&self) -> String {
        "super-whisper-linux".into()
    }

    fn title(&self) -> String {
        "SuperWhisper".into()
    }

    fn icon_name(&self) -> String {
        let state = TrayState::from(self.state.load(Ordering::SeqCst));
        state.icon_name().into()
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        let state = TrayState::from(self.state.load(Ordering::SeqCst));
        ksni::ToolTip {
            title: state.tooltip().into(),
            description: String::new(),
            icon_name: state.icon_name().into(),
            icon_pixmap: Vec::new(),
        }
    }

    fn status(&self) -> ksni::Status {
        let state = TrayState::from(self.state.load(Ordering::SeqCst));
        state.status()
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;

        let state = TrayState::from(self.state.load(Ordering::SeqCst));

        vec![
            StandardItem {
                label: match state {
                    TrayState::Idle => "Start Recording".into(),
                    TrayState::Recording => "Stop Recording".into(),
                    _ => "Toggle".into(),
                },
                icon_name: match state {
                    TrayState::Idle => "media-record".into(),
                    TrayState::Recording => "media-playback-stop".into(),
                    _ => "media-record".into(),
                },
                activate: Box::new(|this: &mut Self| {
                    send_command(&this.socket_path, "toggle");
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Cancel".into(),
                icon_name: "process-stop".into(),
                enabled: state == TrayState::Recording || state == TrayState::Processing,
                activate: Box::new(|this: &mut Self| {
                    send_command(&this.socket_path, "cancel");
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".into(),
                icon_name: "application-exit".into(),
                activate: Box::new(|this: &mut Self| {
                    send_command(&this.socket_path, "shutdown");
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        // Left click toggles recording
        send_command(&self.socket_path, "toggle");
    }
}

fn send_command(socket_path: &str, command: &str) {
    use std::io::Write;
    use std::os::unix::net::UnixStream;

    match UnixStream::connect(socket_path) {
        Ok(mut stream) => {
            if let Err(e) = stream.write_all(format!("{}\n", command).as_bytes()) {
                error!("Failed to send tray command: {}", e);
            }
        }
        Err(e) => {
            error!("Failed to connect to socket for tray command: {}", e);
        }
    }
}

/// System tray icon manager
pub struct TrayIcon {
    handle: TrayHandle,
}

impl TrayIcon {
    /// Create and start a new tray icon
    pub fn new(socket_path: String) -> Result<Self, TrayError> {
        let state = Arc::new(AtomicU8::new(TrayState::Idle as u8));

        let tray = SuperWhisperTray {
            state: state.clone(),
            socket_path,
        };

        let service = ksni::TrayService::new(tray);
        let handle = service.handle();

        // Spawn the tray service in a separate thread (D-Bus needs its own event loop)
        std::thread::spawn(move || {
            if let Err(e) = service.run() {
                error!("Tray service error: {}", e);
            }
        });

        info!("System tray initialized");

        Ok(Self {
            handle: TrayHandle {
                state,
                handle,
            },
        })
    }

    /// Get a handle to control the tray
    pub fn handle(&self) -> TrayHandle {
        self.handle.clone()
    }

    /// Update the tray state
    pub fn set_state(&self, state: TrayState) {
        self.handle.set_state(state);
    }

    /// Get current state
    pub fn state(&self) -> TrayState {
        self.handle.state()
    }
}
