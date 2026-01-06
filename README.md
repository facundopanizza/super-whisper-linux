# SuperWhisper Linux

AI-powered voice-to-text for Linux, inspired by [SuperWhisper](https://superwhisper.com/).
Works with Hyprland and other wlroots-based Wayland compositors.

## Features

- 🎤 Push-to-talk voice recording via hotkey
- 🤖 Local transcription using whisper.cpp (no internet required)
- ☁️ Cloud transcription support (OpenAI, Groq, Deepgram)
- 📋 Auto-paste transcribed text to focused application
- 🌍 Multilingual support (Spanish, English, and more)
- 🖥️ System tray integration

## Requirements

- Linux with Wayland (tested on Hyprland)
- `wl-copy` (wl-clipboard) for clipboard
- `wtype` for paste simulation
- `socat` for IPC communication

## Quick Start

### 1. Build from source

```bash
# Enter development environment
nix develop

# Build release binary
cargo build --release

# Install to ~/.local/bin
cp target/release/super-whisper-linux ~/.local/bin/
```

### 2. Download a Whisper model

```bash
# Create models directory
mkdir -p ~/.local/share/super-whisper-linux/models

# Download base multilingual model (recommended for Spanish/English)
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin \
  -o ~/.local/share/super-whisper-linux/models/ggml-base.bin
```

Available models (larger = more accurate but slower):
- `ggml-tiny.bin` (75 MB) - Fastest, least accurate
- `ggml-base.bin` (142 MB) - Good balance ✓
- `ggml-small.bin` (466 MB) - Better accuracy
- `ggml-medium.bin` (1.5 GB) - High accuracy
- `ggml-large-v3.bin` (3 GB) - Best accuracy

### 3. Create configuration

```bash
# Generate default config
super-whisper-linux init-config

# Or copy the example
cp config.example.toml ~/.config/super-whisper-linux/config.toml
```

### 4. Set up Hyprland keybind

Add to `~/.config/hypr/hyprland.conf`:

```conf
# SuperWhisper - Push to talk
bind = SUPER, V, exec, ~/.local/bin/super-whisper-linux trigger toggle

# Alternative: Use the trigger script
# bind = SUPER, V, exec, ~/Work/super-whisper-linux/scripts/trigger.sh
```

### 5. Start the app

```bash
# Run manually
super-whisper-linux

# Or enable as a systemd user service
cp systemd/super-whisper.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now super-whisper.service
```

## Usage

1. Press `SUPER+V` to start recording (tray icon turns red)
2. Speak your text
3. Press `SUPER+V` again to stop and transcribe
4. Text is automatically pasted to the focused application

### Commands

```bash
# Start the main application
super-whisper-linux

# Trigger commands (when app is running)
super-whisper-linux trigger toggle  # Toggle recording
super-whisper-linux trigger start   # Start recording
super-whisper-linux trigger stop    # Stop and transcribe
super-whisper-linux trigger cancel  # Cancel recording

# Utilities
super-whisper-linux devices         # List audio devices
super-whisper-linux status          # Check app status
super-whisper-linux init-config     # Generate config file
```

## Configuration

Edit `~/.config/super-whisper-linux/config.toml`:

```toml
[general]
default_provider = "whisper-local"  # or "openai", "groq", "deepgram"
language = "auto"                   # or "en", "es", etc.
auto_paste = true                   # Paste after transcription

[audio]
sample_rate = 16000
max_duration = 300                  # Max recording seconds

[providers.whisper-local]
model = "base"                      # tiny, base, small, medium, large

# For cloud providers, set API keys:
# [providers.openai]
# api_key = "sk-..."
```

## Troubleshooting

### App not responding to hotkey
- Check if the socket exists: `ls $XDG_RUNTIME_DIR/super-whisper.sock`
- Check logs: `journalctl --user -u super-whisper -f`

### Paste not working
- Ensure `wtype` is installed: `which wtype`
- Try pasting manually after transcription (text is copied to clipboard)

### No audio recorded
- Check microphone: `super-whisper-linux devices`
- Test with: `arecord -d 3 test.wav && aplay test.wav`

## Development

```bash
# Enter dev environment
nix develop

# Run with debug logging
RUST_LOG=debug cargo run

# Watch for changes
cargo watch -x run
```

## License

MIT
