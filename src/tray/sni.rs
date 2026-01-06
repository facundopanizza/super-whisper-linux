use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;
use tracing::info;

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

/// System tray icon manager
///
/// Note: Full StatusNotifierItem support via ksni requires additional setup.
/// This is a simplified version that tracks state for now.
/// For full tray support, consider using the `tray-icon` crate or implementing
/// the SNI protocol directly.
pub struct TrayIcon {
    state: Arc<AtomicU8>,
}

impl TrayIcon {
    /// Create a new tray icon
    pub fn new() -> Result<Self, TrayError> {
        let state = Arc::new(AtomicU8::new(TrayState::Idle as u8));

        info!("Tray state tracker initialized (full SNI support not yet implemented)");

        Ok(Self { state })
    }

    /// Update the tray icon state
    pub fn set_state(&self, state: TrayState) {
        let old = self.state.swap(state as u8, Ordering::SeqCst);
        if old != state as u8 {
            info!("Tray state: {:?} -> {:?}", TrayState::from(old), state);
        }
    }

    /// Get the current state
    pub fn state(&self) -> TrayState {
        self.state.load(Ordering::SeqCst).into()
    }
}

impl Default for TrayIcon {
    fn default() -> Self {
        Self::new().unwrap()
    }
}
