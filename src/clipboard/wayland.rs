use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tracing::{debug, warn};

use crate::error::ClipboardError;

/// Set text to clipboard using wl-copy (handles Wayland clipboard properly)
pub async fn set_clipboard(text: &str) -> Result<(), ClipboardError> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ClipboardError::SetError(format!("Failed to run wl-copy: {}", e)))?;

    // Write text to stdin
    if let Some(mut stdin) = child.stdin.take() {
        use tokio::io::AsyncWriteExt;
        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|e| ClipboardError::SetError(format!("Failed to write to wl-copy: {}", e)))?;
        // Drop stdin to close it, signaling EOF to wl-copy
        drop(stdin);
    }

    // Wait with timeout to prevent blocking
    match tokio::time::timeout(Duration::from_secs(2), child.wait_with_output()).await {
        Ok(Ok(output)) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(ClipboardError::SetError(format!("wl-copy error: {}", stderr)));
            }
        }
        Ok(Err(e)) => {
            return Err(ClipboardError::SetError(format!("wl-copy failed: {}", e)));
        }
        Err(_) => {
            // Timeout - wl-copy might be forked to hold clipboard, that's OK
            debug!("wl-copy timed out (likely forked to hold clipboard)");
        }
    }

    debug!("Text copied to clipboard ({} chars)", text.len());
    Ok(())
}

/// Paste text to the currently focused application
/// Sets clipboard and simulates Ctrl+V using wtype
pub async fn paste_text(text: &str) -> Result<(), ClipboardError> {
    // First, set the clipboard using wl-copy
    set_clipboard(text).await?;

    // Small delay to ensure clipboard is ready
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Simulate Ctrl+V using wtype
    simulate_paste().await
}

/// Simulate Ctrl+V paste using wtype (for wlroots-based compositors)
async fn simulate_paste() -> Result<(), ClipboardError> {
    // Check if wtype is available (with timeout)
    let wtype_check = tokio::time::timeout(
        Duration::from_secs(1),
        Command::new("which")
            .arg("wtype")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    )
    .await;

    match wtype_check {
        Ok(Ok(status)) if status.success() => {}
        _ => return Err(ClipboardError::WtypeNotFound),
    }

    // Use wtype to simulate Ctrl+V with timeout
    // -M ctrl: hold ctrl modifier
    // -k v: press v key
    // -m ctrl: release ctrl modifier
    let output = tokio::time::timeout(
        Duration::from_secs(2),
        Command::new("wtype")
            .args(["-M", "ctrl", "-k", "v", "-m", "ctrl"])
            .output(),
    )
    .await;

    match output {
        Ok(Ok(output)) if output.status.success() => {
            debug!("Paste simulated with wtype");
            Ok(())
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("wtype failed: {}", stderr);
            // Try alternative: ydotool (requires ydotoold running)
            simulate_paste_ydotool().await
        }
        Ok(Err(e)) => Err(ClipboardError::PasteError(format!("Failed to run wtype: {}", e))),
        Err(_) => {
            warn!("wtype timed out");
            simulate_paste_ydotool().await
        }
    }
}

/// Fallback: simulate paste using ydotool
async fn simulate_paste_ydotool() -> Result<(), ClipboardError> {
    let output = tokio::time::timeout(
        Duration::from_secs(2),
        Command::new("ydotool")
            .args(["key", "29:1", "47:1", "47:0", "29:0"]) // Ctrl down, V down, V up, Ctrl up
            .output(),
    )
    .await;

    match output {
        Ok(Ok(output)) if output.status.success() => {
            debug!("Paste simulated with ydotool");
            Ok(())
        }
        Ok(Ok(output)) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ClipboardError::PasteError(format!(
                "Both wtype and ydotool failed. stderr: {}",
                stderr
            )))
        }
        Ok(Err(e)) => Err(ClipboardError::PasteError(format!("Failed to run ydotool: {}", e))),
        Err(_) => Err(ClipboardError::PasteError("ydotool timed out".into())),
    }
}
