# Winland Server

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Android%208%2B-green.svg)](#)
[![Architecture](https://img.shields.io/badge/arch-arm64-red.svg)](#)
[![Wayland](https://img.shields.io/badge/Wayland-Compositor-orange.svg)](#)
[![Rust](https://img.shields.io/badge/Rust-Core-purple.svg)](#)
[![Kotlin](https://img.shields.io/badge/UI-Kotlin%2FCompose-blueviolet.svg)](#)
[![Build](https://github.com/your-org/winland-android/actions/workflows/android.yml/badge.svg)](.github/workflows/android.yml)

A native Wayland compositor for Android that runs full Linux desktop environments with GPU-accelerated graphics via Vulkan (Zink/Turnip) inside a chroot — no VNC, no SSH, no streaming.

<p align="center">
  <i>XFCE desktop running on LabWC + Wayland, rendered natively on an Android SurfaceView</i>
</p>

---

## Features

### Display & Graphics
- **Native Wayland compositor** built with [Smithay](https://github.com/Smithay/smithay) — no VNC/RDP/streaming overhead
- **Vulkan hardware acceleration** via Zink (OpenGL-on-Vulkan) using Turnip driver on Adreno GPUs
- **GPU acceleration per-app**: Type `gpu` in the terminal followed by the game/app name to launch it with the Zink accelerator enabled
- **OpenGL ES path** via Android HardwareBuffers for devices without Vulkan
- **XWayland** support for legacy X11 applications
- **Dynamic resolution** with persistent scale locking across Android lifecycle events
- **Multi-display ready** architecture

### Input Systems
- **Full touch support** with trackpad emulation — one-finger cursor, two-finger scroll, tap-to-click
- **Relative trackpad emulation** — delta-coordinate gestures for desktop-style cursor control
- **External mouse** — full desktop-grade pointer support via USB/Bluetooth
- **Physical keyboard** — complete keymap handling with XKB, multi-key shortcuts

### Audio (Native)
- **Native audio playback** — Zero-copy PCM audio routed via Oboe (AAudio) between chroot PulseAudio and Android audio hardware through a named FIFO pipe
- **Native microphone input** — Android mic captured via Oboe (AAudio) at 44.1kHz mono 16-bit and streamed into chroot PulseAudio through a dedicated FIFO pipe; fully native with no VNC/streaming overhead

### Clipboard
- **Clipboard sync** — Copy/paste between Android and Linux applications
- **Status**: Partial support (X11 clipboard path under active development, will be fully fixed in a future release)

### Chroot Environment
- **Ubuntu 24.04 (Noble)** and **Kali Nethunter** rootfs support
- **XFCE desktop** session running on **LabWC** (Wayland compositor)
- Automatic rootfs download, extraction, and dependency installation
- BusyBox-based bootstrap for reliable chroot setup

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│                  Android Device                      │
│  ┌──────────────────────────────────────────────┐   │
│  │            Android App (Kotlin)              │   │
│  │  ┌─────────┐ ┌──────────┐ ┌──────────────┐  │   │
│  │  │Dashboard│ │ Terminal │ │  Settings    │  │   │
│  │  │(Compose)│ │ (Termux) │ │  (Compose)   │  │   │
│  │  └────┬────┘ └──────────┘ └──────────────┘  │   │
│  │       │                                      │   │
│  │  ┌────▼──────────────────────────────────┐   │   │
│  │  │         WinlandService                │   │   │
│  │  │  (Foreground Service, Audio Bridge)   │   │   │
│  │  └──────────────────┬───────────────────┘   │   │
│  │                     │                       │   │
│  │  ┌──────────────────▼───────────────────┐   │   │
│  │  │       DisplayActivity                │   │   │
│  │  │  (SurfaceView + Touch/Keyboard)      │   │   │
│  │  └──────────────────┬───────────────────┘   │   │
│  └─────────────────────┼─────────────────────────┘  │
│                        │ UniFFI (JNA)               │
│  ┌─────────────────────▼─────────────────────────┐  │
│  │          Rust Core (libuniffi_winland_core)    │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐   │  │
│  │  │ Wayland  │ │  Input   │ │ Audio/Camera │   │  │
│  │  │Compositor│ │ Bridge   │ │ Bridge       │   │  │
│  │  │(Smithay) │ │          │ │              │   │  │
│  │  └────┬─────┘ └──────────┘ └──────────────┘   │  │
│  └───────┼────────────────────────────────────────┘  │
│          │                                           │
│  ┌───────▼────────────────────────────────────────┐  │
│  │         Linux Chroot (Ubuntu/Kali)             │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────┐   │  │
│  │  │  XFCE    │ │ LabWC    │ │  XWayland    │   │  │
│  │  │ Desktop  │ │(Wayland) │ │  (X11 Apps)  │   │  │
│  │  └──────────┘ └──────────┘ └──────────────┘   │  │
│  │  ┌──────────┐ ┌──────────┐                    │  │
│  │  │ PulseAud.│ │ Zink +   │                    │  │
│  │  │(Audio)   │ │ Turnip   │                    │  │
│  │  └──────────┘ └──────────┘                    │  │
│  └────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────┘
```

### Tech Stack

| Layer | Technology |
|-------|-----------|
| **UI** | Kotlin, Jetpack Compose, Material3 |
| **Compositor** | Rust + Smithay (patched fork) |
| **FFI Bridge** | UniFFI (Mozilla) + JNA |
| **Graphics** | Vulkan (Turnip), OpenGL ES, HardwareBuffer |
| **Input** | Android MotionEvent → Rust compositor |
| **Audio** | Native Oboe (AAudio) — playback FIFO + mic FIFO ↔ PulseAudio inside chroot |
| **Chroot** | BusyBox + shell scripts |
| **Compiler** | Rust → NDK (aarch64-linux-android), Meson (C libs), Gradle (Kotlin) |

---

## Requirements

| Requirement | Details |
|------------|---------|
| **Root** | Magisk or KernelSU (for chroot mounts) |
| **Device** | ARM64 (arm64-v8a) |
| **Android** | API 26+ (Android 8.0 Oreo) |
| **GPU** | Adreno 650+ recommended (Vulkan 1.1+ for Turnip) |
| **Storage** | 8 GB+ free for rootfs + apps |
| **Optional** | USB/Bluetooth mouse & keyboard |

---

## Installation

### Pre-built APK
1. Download the latest APK from [Releases](https://github.com/your-org/winland-android/releases)
2. Install on your device (enable "Install from unknown sources")
3. Open the app → grant root permission → select your distro → **Install**
4. Wait for rootfs download + setup
5. Press **Run** — XFCE desktop starts in a native Wayland session

### Build from Source

#### Prerequisites
- Android SDK 34 + NDK 26 (or later)
- Rust toolchain with `aarch64-linux-android` target
- Meson + Ninja (for libxkbcommon)

#### Quick Build

```bash
# Clone
git clone https://github.com/your-org/winland-android.git
cd winland-android

# Set up environment
cp .env.example .env
# Edit .env with your SDK/NDK paths

# Full ARM64 build (Rust + C + Gradle)
bash build-arm64.sh

# APK will be at:
# app/build/outputs/apk/debug/app-debug.apk
```

#### Step-by-Step

```bash
# 1. Build libxkbcommon (C library for keyboard)
cd libxkbcommon
meson setup build-android --cross-file android-arm64.ini
ninja -C build-android libxkbcommon.so
cd ..

# 2. Build terminal JNI library
cd terminal-emulator/src/main
ndk-build NDK_PROJECT_PATH=. APP_ABI=arm64-v8a APP_PLATFORM=android-31
cd ../..

# 3. Build Rust core library
export CC_aarch64_linux_android=/path/to/ndk/aarch64-linux-android31-clang
export CXX_aarch64_linux_android=/path/to/ndk/aarch64-linux-android31-clang++
cd native
cargo build --release --lib --target aarch64-linux-android --features smithay_android
cd ..

# 4. Copy native libs
cp native/target/aarch64-linux-android/release/libuniffi_winland_core.so app/src/main/jniLibs/arm64-v8a/
cp terminal-emulator/src/main/libs/arm64-v8a/libtermux.so app/src/main/jniLibs/arm64-v8a/

# 5. Build Android APK
./gradlew assembleDebug
```

---

## How It Works

1. **Bootstrapping**: The app downloads a compressed Linux rootfs (Ubuntu or Kali) and extracts it into the app's private data directory.

2. **Chroot Setup**: With root privileges, the app mounts `/proc`, `/sys`, `/dev`, and bind-mounts the Wayland socket and audio FIFO into the chroot.

3. **Compositor**: The Rust-based Smithay compositor creates a Wayland socket inside the app's data directory. This socket is bind-mounted into the chroot so Linux GUI apps can connect to it.

4. **Desktop Launch**: Inside the chroot, LabWC (a Wayland compositor) starts as the window manager, XFCE as the desktop environment, and XWayland for X11 app compatibility.

5. **Input Routing**: Touch events, keyboard input, and mouse events from Android are forwarded through UniFFI to the Rust compositor, which injects them into the Wayland protocol.

6. **Audio Bridge (Native)**: PulseAudio inside the chroot writes PCM audio to a named FIFO; the Rust audio bridge reads it via Oboe (AAudio) for zero-copy playback on Android speakers. For microphone input, Oboe captures 44.1kHz mono 16-bit audio from the Android mic and writes to a second FIFO, which PulseAudio reads as the `AndroidMic` source — all natively with no streaming overhead.

7. **Rendering**: The compositor renders directly onto an Android `SurfaceView` using Vulkan (Zink/Turnip) or OpenGL ES via HardwareBuffers.

---

## Configuration

### Environment Variables
Create a `.env` file in the project root (see `.env.example`):

| Variable | Default | Description |
|----------|---------|-------------|
| `ANDROID_SDK_ROOT` | `$HOME/Android/Sdk` | Android SDK path |
| `ANDROID_NDK_VERSION` | `25.2.9519653` | NDK version |
| `RUST_TARGET` | `aarch64-linux-android` | Rust compilation target |
| `BUILD_JOBS` | `4` | Parallel build jobs |
| `ENABLE_LTO` | `true` | Link-time optimization |
| `ENABLE_NEON` | `true` | NEON SIMD for ARM64 |

### Runtime Toggles
- **GPU Acceleration**: Set `MESA_LOADER_DRIVER_OVERRIDE=zink` to enable Vulkan-to-OpenGL translation
- **Input Mode**: Switch between direct touch, trackpad, and external mouse in the app's input settings
- **Desktop Scale**: Adjustable in settings (locked across Android lifecycle restarts)

---

## Project Structure

```
├── app/                          # Android app module (Kotlin)
│   ├── src/main/assets/          # Shell scripts + BusyBox
│   ├── src/main/kotlin/          # UI, engine, services
│   ├── src/main/jniLibs/         # Prebuilt native .so files
│   └── build.gradle.kts          # App build config
├── native/                       # Rust compositor core
│   ├── src/                      # Wayland compositor, audio, input, camera
│   │   ├── android/              # Android JNI bridge modules
│   │   └── core/                 # Core utilities
│   ├── Cargo.toml
│   └── lib/                      # Patched forks (Smithay, Winit)
├── terminal-emulator/            # Termux terminal emulator (Java + C)
├── terminal-view/                # Terminal View (Android UI)
├── libxkbcommon/                 # XKB keyboard library (C)
├── build-arm64.sh                # Full build script
└── .github/workflows/android.yml # CI pipeline
```

---

## Contributing

Contributions are welcome — whether it's fixing bugs, adding features, or improving documentation.

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-idea`)
3. Commit your changes (`git commit -m 'Add amazing idea'`)
4. Push to the branch (`git push origin feature/amazing-idea`)
5. Open a Pull Request

### Development Tips
- The Rust compositor is the most performance-critical component — profile carefully
- Use `build-arm64.sh` for a full rebuild (Rust → C → Kotlin)
- For Kotlin-only changes, `./gradlew assembleDebug` is sufficient if native libs are prebuilt
- Debug logs: `logcat -s WinlandServer WinlandNative`

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

### Third-Party Components
| Component | License |
|-----------|---------|
| [Smithay](https://github.com/Smithay/smithay) | MIT |
| [Winit](https://github.com/rust-windowing/winit) | Apache 2.0 / MIT |
| [libxkbcommon](https://github.com/xkbcommon/libxkbcommon) | MIT / Public Domain |
| [Termux](https://github.com/termux/termux-app) | GPL-3.0 |
| [UniFFI](https://github.com/mozilla/uniffi-rs) | MPL-2.0 |
| [BusyBox](https://busybox.net/) | GPL-2.0 |

---

## Acknowledgments

- The [Smithay](https://github.com/Smithay/smithay) project for the Wayland compositor library
- The [Turnip](https://gitlab.freedesktop.org/mesa/mesa/-/tree/master/src/gallium/drivers/zink) driver team for Vulkan-on-Adreno support
- [Termux](https://github.com/termux/termux-app) for the terminal emulator
- The Android Linux chroot community for pioneering the approach

---

<p align="center">
  <sub>Built with ❤️ for the Android Linux enthusiast community</sub>
</p>
