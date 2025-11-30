# Clipper

> **A "TikTok-style" video recorder for Desktop. Built with Rust.**

![Rust](https://img.shields.io/badge/Made%20with-Rust-orange?style=for-the-badge&logo=rust)
![Platform](https://img.shields.io/badge/Platform-Windows%20|%20Linux-blue?style=for-the-badge)
![License](https://img.shields.io/badge/License-GPL3-green?style=for-the-badge)

**Clipper** is a high-performance, native video recording tool designed for rapid content creation. Forget complex timelines and rendering queues. Just **Hold-to-Record** and **Release-to-Pause**. When you're done, Clipper instantly stitches your segments into a seamless MP4.

Powered by **Rust**, **FFmpeg**, and **egui**.

---

## Features

*   **Non-Linear Recording:** Hold `Space` to record, release to stop. Press again to append the next clip instantly.
*   **Instant Undo:** Hit `Backspace` to delete just the last segment and keep going.
*   **Hardware Acceleration:** Native support for **NVIDIA (NVENC)**, **AMD (AMF)**, and **Intel (QuickSync)** encoding.
*   **Zero-Copy Pipeline:** Optimized architecture pipes raw MJPEG/YUYV data directly from the camera to the encoder to minimize CPU usage.
*   **Full Control:** Select your resolution, framerate, and encoding quality (High/Medium/Low).
*   **Multi-Threaded:** UI, Camera Capture, and Video Encoding run on separate threads for smooth 60fps performance.

---

## Prereqs

Clipper uses **FFmpeg** as a sidecar process to handle video encoding. You **must** have FFmpeg installed and added to your system PATH.

### Windows
1.  Open PowerShell as Administrator.
2.  Run: `winget install Gyan.FFmpeg`
3.  **Restart your terminal/IDE**

### Linux
```bash
# Ubuntu / Debian
sudo apt update && sudo apt install ffmpeg

# Fedora
sudo dnf install ffmpeg

# Arch
sudo pacman -S ffmpeg
```

**Verification:** Open a terminal and type `ffmpeg -version`. If you see version info you're all set.

---

## Getting Started

1.  **Clone the repo:**
    ```bash
    git clone https://github.com/yourusername/clipper.git
    cd clipper
    ```

2.  **Run in Release Mode:**
    **Important:** Always run video apps in release mode. Debug mode is too slow for real-time 4K processing.
    ```bash
    cargo run --release
    ```

---

## Controls

Clipper is designed to be controlled by your keyboard for a tactile recording experience.

| Key | Action | Description |
| :--- | :--- | :--- |
| **Spacebar (Hold)** | **Record** | Records video while held down. |
| **Spacebar (Release)** | **Pause** | Stops recording and saves the segment. |
| **Backspace** | **Undo** | Deletes the most recent segment. |
| **Enter** | **Finish** | Stitches all segments into `output.mp4`. |

---

## Architecture

Clipper is a multi-threaded pipeline designed for throughput.

1.  **Camera Thread (`camera.rs`):** 
    *   Captures frames using `nokhwa`.
    *   **Optimization:** Splits the data immediately. It sends the **Raw Buffer** (MJPEG/YUYV) to the recorder (fast) and decodes a **Downscaled Copy** (RGB) for the UI preview.
2.  **Recorder Thread (`recorder/`):** 
    *   Receives raw bytes and pipes them into a persistent `ffmpeg` process via `stdin`.
    *   Manages the playlist of temporary `.mp4` segments.
    *   Uses the **Concat Demuxer** to merge files instantly without re-encoding.
3.  **UI Thread (`app.rs`):** 
    *   Built with `egui`. Renders the preview texture and listens for keyboard events.
    *   Completely decoupled from the recording logic—if the UI hangs, the recording keeps going.

---

## Roadmap

*   [x] Variable Frame Rate support (Fix "speed up" issues)
*   [x] Hardware Encoding support
*   [ ] **Audio Capture** (Syncing microphone input with video segments)
*   [ ] Custom Output Filenames
*   [ ] Drag-and-drop segment reordering

---

## License

Distributed under version 3 of the GNU General Public License. See `LICENSE` for more information.