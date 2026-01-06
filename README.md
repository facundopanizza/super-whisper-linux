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

## Installation

### NixOS / Home-Manager (Recommended)

Add to your `flake.nix` inputs:

```nix
{
  inputs = {
    super-whisper-linux = {
      url = "github:facundopanizza/super-whisper-linux";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

Then in your home-manager config (`home.nix`):

```nix
{ inputs, pkgs, ... }:

{
  home.packages = [
    inputs.super-whisper-linux.packages.${pkgs.system}.default
  ];
}
```

### Nix (without flakes)

```bash
nix-env -if https://github.com/facundopanizza/super-whisper-linux/archive/main.tar.gz
```

### Build from source

```bash
# Enter development environment
nix develop

# Build release binary
cargo build --release

# Install to ~/.local/bin
cp target/release/super-whisper-linux ~/.local/bin/
```

## Quick Start

### 1. Download a Whisper model

```bash
# Using the built-in download command (recommended)
super-whisper-linux download-model --model base

# Or manually with curl
mkdir -p ~/.local/share/super-whisper-linux/models
curl -L https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin \
  -o ~/.local/share/super-whisper-linux/models/ggml-base.bin
```

Available models (larger = more accurate but slower):
| Model | Size | Speed | Accuracy |
|-------|------|-------|----------|
| tiny | 75 MB | Fastest | Basic |
| base | 142 MB | Fast | Good ✓ |
| small | 466 MB | Medium | Better |
| medium | 1.5 GB | Slow | High |
| large | 3 GB | Slowest | Best |

### 2. Create configuration

```bash
super-whisper-linux init-config
```

### 3. Set up Hyprland keybind

Add to `~/.config/hypr/hyprland.conf`:

```conf
# SuperWhisper - Push to talk
bind = SUPER, B, exec, super-whisper-linux trigger toggle
```

### 4. Start the app

```bash
# Run manually
super-whisper-linux

# Or add to Hyprland autostart
# In hyprland.conf:
exec-once = super-whisper-linux
```

## Usage

1. Press `SUPER+B` to start recording
2. Speak your text
3. Press `SUPER+B` again to stop and transcribe
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

# Model management
super-whisper-linux download-model --model base  # Download a model

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
- Check if app is running: `super-whisper-linux status`

### Paste not working
- Ensure `wtype` is installed and in PATH
- Try pasting manually after transcription (text is copied to clipboard)

### No audio recorded
- Check microphone: `super-whisper-linux devices`
- Test with: `arecord -d 3 test.wav && aplay test.wav`

## Development

```bash
# Enter dev environment
nix develop

# Run with debug logging
cargo run -- --debug

# Watch for changes
cargo watch -x run
```

## License

MIT
