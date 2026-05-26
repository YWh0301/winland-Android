# Winland Server 🐧📱

**Wayland Compositor for Android**

Winland Server is a full-featured Wayland compositor that runs on Android devices, allowing you to run Linux GUI applications directly on your phone or tablet.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Platform](https://img.shields.io/badge/platform-Android-green.svg)
![Version](https://img.shields.io/badge/version-1.0.0-orange.svg)

⚠️ Requirements
​Root Access Required: This server operates inside a chroot/proot Linux environment and interacts deeply with Android system processes. Magisk or KernelSU is strictly required to bind mount points and manage environment sockets.
​✨ Supported Features
​🖥️ Display & Rendering
​Full App Compatibility: Native support for running modern Wayland clients alongside legacy XWayland (X11) applications seamlessly.
​Efficient Software Rendering (shm): Utilizing Shared Memory (wl_shm) and the stable Pixman CPU renderer, ensuring consistent performance without relying on restricted hardware DRM file descriptors.
​Dynamic Resolution & Scaling: Full support for updating resolution layouts with an architectural 3-Guard persistence layer that prevents Android lifecycles from resetting custom scales (e.g., locking a crisp 720p view on 1080p screens).
​🎛️ Input Systems & Control
​Exclusive Multi-Mode Input: A hardware-grade toggle system that prevents event multiplexing:
​Direct Touch Mapping: Pure absolute layout translation for standard mobile use.
​Relative Trackpad Emulation: Advanced gesture/cursor control driven by delta coordinates (dx/dy) to provide a laptop-like trackpad experience.
​External Physical Mouse: Full desktop-grade support for connected USB/Bluetooth mice.
​Keyboard & Shortcuts Control: Robust physical keyboard support including system-wide multi-key desktop shortcuts and binds.
​🧪 Experimental & Partial Features
​Shared Clipboard (Text Only): Initial, basic support for syncing text clipboards between Android and Linux apps (file synchronization is still under active development).
​Basic Audio Routing: Early-stage, experimental bridging for application audio output.

