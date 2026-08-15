<p align="center">
  <img src="assets/logo.svg" alt="MovingText logo" width="480">
</p>

<p align="center">
  <a href="https://github.com/imnakul/movingtext/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/imnakul/movingtext/ci.yml?branch=main&label=build&style=flat-square" alt="Build status"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square" alt="License: MIT"></a>
  <img src="https://img.shields.io/badge/platform-Windows%2010%2F11-0A0A0D.svg?style=flat-square" alt="Platform: Windows">
  <img src="https://img.shields.io/badge/built%20with-Rust-orange.svg?style=flat-square" alt="Built with Rust">
</p>

## Overview

MovingText is a high-performance Windows desktop enhancement suite built in Rust. It combines a customizable scrolling edge marquee for ambient reminders with an interactive Dynamic Island (Notch) HUD for quick status, media playback controls, clock display, and desktop utilities.

The application operates locally with zero telemetry, zero cloud dependencies, and minimal system resource footprint, rendering via hardware-accelerated Direct2D and DirectWrite.

## Core Features

### 1. Dynamic Island / Notch HUD
- **Interactive Bezel Panel**: Sits flush at the top or bottom of your monitor, expanding smoothly on hover with spring physics animations.
- **Carousel Slides**:
  - **Status Slide**: Displays daily focus priorities and checklist items with clean status pills.
  - **Clock Slide**: Oversized digital clock with 12-hour or 24-hour formats and date presentation.
  - **Media Controls Slide**: Windows System Media Transport Controls (SMTC) integration displaying current track title, artist, live playback status, and interactive controls (play, pause, next, previous).
  - **Photo and Visual Slide**: Framed wallpaper preview and image rendering via Windows Imaging Component (WIC).
  - **Marquee Quick Slide**: Displays active scrolling message text inside the notch.
- **Pin Mode**: Lock the island in an expanded state so it remains open while you work.
- **Mouse Wheel Navigation**: Scroll over the notch to cycle through slides seamlessly.
- **Click-Through Option**: Optional pass-through mode allows mouse clicks to reach underlying applications while preserving hover activation.

### 2. Glass Finishes and Surface Themes
- **Frosted Glass**: Balanced live background blur with specular rim highlight and subtle translucent tinting.
- **Transparent (Clear Glass)**: Crystal-clear see-through glass with zero blur, subtle specular borders, and high-contrast typography.
- **Blurred (Heavy Diffusion)**: Deep Gaussian-style diffusion blur that blends background windows into smooth ambient gradients.
- **Acrylic**: Windows Fluent Acrylic style with balanced diffusion and rich ambient tinting.
- **Dark (Obsidian)**: Deep dark panel engineered to blend into physical laptop bezels and dark wallpapers.
- **Light (Bone White)**: Clean white panel with dark typography for light desktop environments.
- **DWM Capture Exclusion**: Prevents recursive capture feedback loops by isolating the notch from desktop sampling during live blur rendering.

### 3. Scrolling Edge Marquee
- **Multi-Edge Support**: Run the scrolling marquee along any combination of the Top, Bottom, Left, and Right screen edges.
- **Typography and Script Support**: Full Unicode rendering via DirectWrite supporting Latin, Devanagari, CJK scripts, and symbols.
- **Geometry Controls**: Configurable strip thickness, edge clearance padding (to avoid covering taskbars or window title bars), phrase spacing, and scroll speed/direction.
- **Window Layering**: Full Always-on-Top support and transparent click-through mode.

### 4. Custom Modern Color Panel
- **Figma-Grade Color Editor**: Replaces stock color pickers with a dedicated, precision color management suite.
- **Interactive Trigger Capsule**: Displays real-time color swatches with checkerboard transparency underlays and hex code indicators.
- **2D Saturation/Value Canvas**: Hardware-accelerated bilinear gradient mesh with dual-ring precision reticle.
- **Spectrum Hue and Alpha Sliders**: Continuous 12-segment rainbow spectrum slider and transparency slider with live percentage readouts.
- **Multi-Format Input Modes**:
  - **HEX**: Hex code input with quick-copy clipboard button and opacity drag values.
  - **RGB**: Individual 0 to 255 channels for Red, Green, Blue, and Alpha.
  - **HSV**: Intuitive degree and percentage channels (0 to 360 degrees Hue, 0 to 100 percent Saturation and Value).
- **Curated Swatches Palette**: Instant one-click presets covering modern dark tones, light neutrals, and vivid accents.

### 5. Theme and Wallpaper Engine
- **Wallpaper Color Extraction**: Samples current desktop wallpaper or imported images to extract dominant accent tones.
- **Adaptive UI Themes**: Switch settings UI between System, Light, and Dark palettes with smooth 200ms cross-fade animations.
- **Native File Dialog**: Import custom images and wallpapers via native Windows Shell API.

### 6. System Tray and Window Management
- **Background Execution**: Closing or hiding the settings window minimizes the application to the Windows notification area (System Tray).
- **Quick Tray Menu**: Left-click to open settings, right-click to access quick toggles for the Notch, Edge Marquee, or application exit.
- **Single-Instance Management**: Ensures reliable window restoration and foreground focusing without duplicate processes.

### 7. Notification Hub and Agent Hooks
- **Live Desktop Alerts**: Dynamic toast notifications expand smoothly out of the collapsed notch with spring physics.
- **Notifications Slide**: Dedicated notification center slide with unread badges, card inspection, individual dismissal, and batch clear options.
- **Developer and AI Agent Integration**: CLI scripts in `scripts/notch-hooks/` enable external tools, build scripts, and AI agents (such as Antigravity, Claude Code, and Codex) to deliver instant desktop HUD alerts directly to the notch.

## How to Use Features

### Using the Dynamic Island (Notch)
1. **Expanding the Panel**: Move your mouse cursor to the top center of your screen. The notch will smoothly expand.
2. **Switching Slides**: Use your mouse scroll wheel while hovering over the notch to walk through active slides (Status, Clock, Media, Photo, Marquee).
3. **Pinning Open**: Click the small circle icon on the right side of the expanded panel to pin it open. Click again to unpin and allow auto-collapse on mouse leave.
4. **Controlling Media**: When music or videos are playing on Windows, switch to the Media slide to control playback or view track titles and artists.
5. **Adjusting Placement**: In Settings > Notch, change the alignment (Top or Bottom), monitor target, width, height, and vertical offset.

### Configuring Edge Marquee
1. Open Settings > Text to enter your custom reminder, quote, or note.
2. Open Settings > Appearance to toggle which screen edges are active (Top, Bottom, Left, Right).
3. Adjust thickness and clearance padding so the marquee sits neatly outside your taskbar and window borders.
4. Set custom colors and opacity using the Color Panel.
5. In Settings > Behavior, adjust scroll speed, direction (forward/reverse), and click-through options.

### Customizing Colors and Themes
1. Open any color setting (e.g. Accent Color, Panel Color, Marquee Text Color).
2. Click the color capsule to reveal the modern Color Panel.
3. Drag inside the 2D gradient box to adjust saturation and brightness.
4. Slide the rainbow bar to change Hue, and slide the checkerboard bar to set opacity.
5. Switch between HEX, RGB, and HSV tabs to enter exact values or click a preset swatch.

## Installation

### Option 1: Download Release Binary
Download the latest `movingtext.exe` from the GitHub Releases page and run it directly. No installer or administrative permissions required.

### Option 2: Build from Source
Requires the [Rust toolchain](https://www.rust-lang.org/tools/install) (stable) on Windows 10 or Windows 11.

```bash
git clone https://github.com/imnakul/movingtext.git
cd movingtext
cargo build --release
```

The compiled binary will be located at `target/release/movingtext.exe`.

## Configuration Storage

All preferences and states are saved automatically to a local JSON file at:

```
%APPDATA%\movingtext\config.json
```

To reset the application to factory defaults, simply delete this file while the application is closed.

## Architecture and Technology Stack

- **Language**: Rust (2021 Edition)
- **GUI Framework**: egui and eframe for the settings interface
- **Graphics Pipeline**: Direct2D and DirectWrite via Microsoft Windows-rs bindings
- **Backdrop Compositor**: GDI screen capture with DWM capture exclusion and bilinear Direct2D interpolation
- **Media Integration**: Windows System Media Transport Controls (SMTC / WinRT)
- **Typography**: Plus Jakarta Sans and Noto Sans Devanagari (SIL Open Font License)

## Contributing

Contributions and bug reports are welcome. Before submitting pull requests, ensure your changes compile with zero warnings:

```bash
cargo fmt --check
cargo check
```

## License

MovingText is licensed under the [MIT License](LICENSE).
