use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
use std::sync::mpsc;
use std::sync::{RwLock, OnceLock};

/// A raw ANativeWindow pointer that is safe to send between threads
/// (Android's ANativeWindow is reference-counted and thread-safe).
#[derive(Debug, Clone, Copy)]
pub struct NativeWindowPtr(pub *mut std::ffi::c_void);
unsafe impl Send for NativeWindowPtr {}
unsafe impl Sync for NativeWindowPtr {}

/// Commands sent from JNI threads to the compositor thread.
/// The compositor thread processes these in its main loop,
/// owning all state directly (no Mutex locks).
#[derive(Debug)]
pub enum JniCommand {
    // ── Input events (fire-and-forget) ──
    TouchInput { action: i32, id: i32, x: f32, y: f32 },
    KeyInput { keycode: i32, is_down: bool },
    TextInput { text: String },
    /// Trackpad: relative cursor motion (dx/dy in view pixels, time in ms).
    /// Used only in exclusive Trackpad mode.
    RelativeMotion { dx: f32, dy: f32, time: u32 },
    /// Trackpad: click event (state: 1=pressed, 0=released, button: 0x110=left, 0x111=right).
    /// Used only in exclusive Trackpad mode.
    TrackpadClick { state: i32, button: i32, time: u32 },
    /// Trackpad: continuous two-finger scroll, or the terminating axis-stop frame.
    TrackpadScroll { dx: f32, dy: f32, time: u32, finished: bool },
    /// Enable bounded input-to-presentation diagnostic correlation.
    SetInputLatencyTrace { enabled: bool },

    // ── Lifecycle (with response) ──
    ShutdownCompositor {
        response: mpsc::Sender<()>,
    },

    // ── EGL / Surface (with response) ──
    BindNativeWindow {
        native_window: NativeWindowPtr,
        response: mpsc::Sender<Result<(), String>>,
    },
    ReleaseNativeWindow {
        response: mpsc::Sender<()>,
    },
    SurfaceChanged {
        width: i32,
        height: i32,
        physical_width_mm: i32,
        physical_height_mm: i32,
    },

    // ── Config (fire-and-forget) ──
    SetInputMode { mode: i32 },
    SetScrollSensitivity { value: f32 },
    SetRefreshRate { value: f32 },
    SetResolution { width: i32, height: i32 },
    /// Change wl_output.scale only. logical_size always equals surface_size.
    /// Used by resolution preset chips (1080p → 1.0, large UI → 1.5).
    SetScale { scale: f32 },
    EnableShm,
    DisableShm,
    EnableDmabuf,
    SetRelativeSensitivity { value: f32 },
    SetPhysicalSize { width_mm: i32, height_mm: i32 },
    SetYOffset { y_offset: i32 },
    SuspendRendering,
    ResumeRendering,
    UpdateClipboard { text: String },
    /// Clipboard text flowing Wayland/X11 → Android.
    /// Set from background threads, processed on compositor thread,
    /// picked up by Kotlin polling via getWaylandClipboardGen / pollWaylandClipboard.
    WaylandClipboardToAndroid { text: String },

    // ── Queries (with response) ──
    GetRuntimeStats {
        response: mpsc::Sender<String>,
    },
    GetSurfaceSize {
        response: mpsc::Sender<(i32, i32)>,
    },
    GetScrollSensitivity {
        response: mpsc::Sender<f32>,
    },
    GetBackendSnapshot {
        response: mpsc::Sender<String>,
    },
}

/// Global sender for dispatching commands to the compositor thread.
pub static COMMAND_TX: OnceLock<crossbeam_channel::Sender<JniCommand>> = OnceLock::new();

/// Set the global command channel sender (called once when the compositor spawns).
pub fn set_command_tx(tx: crossbeam_channel::Sender<JniCommand>) {
    let _ = COMMAND_TX.set(tx);
}

/// Send a command to the compositor thread. Returns `false` if the channel is not set up yet.
pub fn send_command(cmd: JniCommand) -> bool {
    COMMAND_TX.get().map(|tx| tx.send(cmd).is_ok()).unwrap_or(false)
}

/// Check whether the command channel is initialized.
pub fn is_initialized() -> bool {
    COMMAND_TX.get().is_some()
}

/// Cached flag: whether any Wayland clients are currently connected to the compositor.
/// Updated by the compositor thread each frame (no channel round-trip needed for reads).
static CLIENTS_CONNECTED: AtomicBool = AtomicBool::new(false);

/// Set the client-connected flag (called from compositor thread).
pub fn set_clients_connected(connected: bool) {
    CLIENTS_CONNECTED.store(connected, Ordering::Relaxed);
}

/// Read whether clients are connected (called from JNI / main thread).
pub fn are_clients_connected() -> bool {
    CLIENTS_CONNECTED.load(Ordering::Relaxed)
}

/// XWayland display number, set from Kotlin via JNI after the chroot starts.
/// Initialized to -1 meaning "not yet known".
static XWAYLAND_DISPLAY: AtomicI32 = AtomicI32::new(-1);

/// X11 socket directory (e.g. "/data/data/com.winland.server/files/tmp"),
/// set from Kotlin via JNI. Used to construct the XWayland connect path.
static X11_SOCKET_DIR: OnceLock<RwLock<String>> = OnceLock::new();

pub fn set_x11_socket_dir(dir: String) {
    let cache = X11_SOCKET_DIR.get_or_init(|| RwLock::new(String::new()));
    if let Ok(mut guard) = cache.write() {
        log::info!("X11 socket dir set to '{}'", dir);
        *guard = dir;
    }
}

pub fn get_x11_socket_dir() -> String {
    let cache = X11_SOCKET_DIR.get_or_init(|| RwLock::new(String::new()));
    if let Ok(guard) = cache.read() {
        guard.clone()
    } else {
        String::new()
    }
}

/// Set the XWayland display number (called from JNI / Kotlin).
pub fn set_xwayland_display(display: i32) {
    XWAYLAND_DISPLAY.store(display, Ordering::Release);
    log::info!("XWayland display number set to :{}", display);
}

/// Read the XWayland display number (called from compositor thread).
/// Returns `-1` if not yet set.
pub fn get_xwayland_display() -> i32 {
    XWAYLAND_DISPLAY.load(Ordering::Acquire)
}

// ── Runtime stats cache ───────────────────────────────────────────────────────
// Cached copy of Wayland runtime stats, updated by the compositor thread each
// frame. JNI reads this instantly with zero channel round-trip or lock contention.

static RUNTIME_STATS_CACHE: OnceLock<RwLock<String>> = OnceLock::new();

/// Update the cached runtime stats string (called from compositor thread).
pub fn set_runtime_stats(stats: String) {
    let cache = RUNTIME_STATS_CACHE.get_or_init(|| RwLock::new(String::new()));
    if let Ok(mut guard) = cache.write() {
        *guard = stats;
    }
}

/// Read the cached runtime stats string (called from JNI / main thread).
pub fn get_runtime_stats() -> String {
    let cache = RUNTIME_STATS_CACHE.get_or_init(|| RwLock::new(String::new()));
    if let Ok(guard) = cache.read() {
        guard.clone()
    } else {
        "cache_unavailable".to_string()
    }
}

// ── Backend state caches ──────────────────────────────────────────────────────
// Cached copies of AndroidSmithayState fields, updated by the compositor thread
// each frame. JNI reads these instantly with zero channel round-trip or lock
// contention.

static CACHED_SURFACE_SIZE: OnceLock<RwLock<(i32, i32)>> = OnceLock::new();
static CACHED_LOGICAL_SIZE: OnceLock<RwLock<(i32, i32)>> = OnceLock::new();
static CACHED_SCALE: AtomicU32 = AtomicU32::new(f32::to_bits(1.0));
static CACHED_SCROLL_SENSITIVITY: OnceLock<RwLock<f32>> = OnceLock::new();
static CACHED_BACKEND_SNAPSHOT: OnceLock<RwLock<String>> = OnceLock::new();
static CACHED_PHYSICAL_SIZE: OnceLock<RwLock<(i32, i32)>> = OnceLock::new();

pub fn set_surface_size(w: i32, h: i32) {
    let cache = CACHED_SURFACE_SIZE.get_or_init(|| RwLock::new((0, 0)));
    if let Ok(mut guard) = cache.write() {
        *guard = (w, h);
    }
}

pub fn get_surface_size() -> (i32, i32) {
    let cache = CACHED_SURFACE_SIZE.get_or_init(|| RwLock::new((0, 0)));
    if let Ok(guard) = cache.read() {
        *guard
    } else {
        (0, 0)
    }
}

pub fn set_logical_size(w: i32, h: i32) {
    let cache = CACHED_LOGICAL_SIZE.get_or_init(|| RwLock::new((0, 0)));
    if let Ok(mut guard) = cache.write() {
        *guard = (w, h);
    }
}

pub fn get_logical_size() -> (i32, i32) {
    let cache = CACHED_LOGICAL_SIZE.get_or_init(|| RwLock::new((0, 0)));
    if let Ok(guard) = cache.read() {
        *guard
    } else {
        (0, 0)
    }
}

pub fn set_scale(val: f32) {
    CACHED_SCALE.store(val.to_bits(), std::sync::atomic::Ordering::Relaxed);
}

pub fn get_scale() -> f32 {
    f32::from_bits(CACHED_SCALE.load(std::sync::atomic::Ordering::Relaxed))
}

pub fn set_scroll_sensitivity(v: f32) {
    let cache = CACHED_SCROLL_SENSITIVITY.get_or_init(|| RwLock::new(1.0));
    if let Ok(mut guard) = cache.write() {
        *guard = v;
    }
}

pub fn get_scroll_sensitivity() -> f32 {
    let cache = CACHED_SCROLL_SENSITIVITY.get_or_init(|| RwLock::new(1.0));
    if let Ok(guard) = cache.read() {
        *guard
    } else {
        1.0
    }
}

pub fn set_backend_snapshot(s: String) {
    let cache = CACHED_BACKEND_SNAPSHOT.get_or_init(|| RwLock::new(String::new()));
    if let Ok(mut guard) = cache.write() {
        *guard = s;
    }
}

pub fn get_backend_snapshot() -> String {
    let cache = CACHED_BACKEND_SNAPSHOT.get_or_init(|| RwLock::new(String::new()));
    if let Ok(guard) = cache.read() {
        guard.clone()
    } else {
        "backend cache unavailable".to_string()
    }
}

pub fn set_physical_size(w: i32, h: i32) {
    let cache = CACHED_PHYSICAL_SIZE.get_or_init(|| RwLock::new((155, 87)));
    if let Ok(mut guard) = cache.write() {
        *guard = (w.max(1), h.max(1));
    }
}

pub fn get_physical_size() -> (i32, i32) {
    let cache = CACHED_PHYSICAL_SIZE.get_or_init(|| RwLock::new((155, 87)));
    if let Ok(guard) = cache.read() {
        *guard
    } else {
        (155, 87)
    }
}

#[cfg(all(test, feature = "smithay_android"))]
mod tests {
    use super::*;

    #[test]
    fn cache_defaults() {
        assert_eq!(get_surface_size(), (0, 0));
        assert_eq!(get_scroll_sensitivity(), 1.0);
        assert_eq!(get_backend_snapshot(), "");
        assert_eq!(get_physical_size(), (155, 87));
    }

    #[test]
    fn cache_write_then_read() {
        set_surface_size(1920, 1080);
        assert_eq!(get_surface_size(), (1920, 1080));

        set_logical_size(1080, 2233);
        assert_eq!(get_logical_size(), (1080, 2233));

        set_scroll_sensitivity(2.5);
        assert_eq!(get_scroll_sensitivity(), 2.5);

        set_backend_snapshot("test_snapshot".to_string());
        assert_eq!(get_backend_snapshot(), "test_snapshot");

        set_physical_size(300, 200);
        assert_eq!(get_physical_size(), (300, 200));
    }

    #[test]
    fn scale_cache_default() {
        assert!((get_scale() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scale_cache_write_read() {
        set_scale(1.5);
        assert!((get_scale() - 1.5).abs() < f32::EPSILON);
        set_scale(1.0);
        assert!((get_scale() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn cache_overwrite() {
        set_surface_size(800, 600);
        set_surface_size(1024, 768);
        assert_eq!(get_surface_size(), (1024, 768));
    }

    #[test]
    fn crossbeam_channel_send_recv_roundtrip() {
        let (tx, rx) = crossbeam_channel::unbounded::<JniCommand>();

        tx.send(JniCommand::EnableShm).unwrap();
        let received = rx.try_recv().unwrap();
        assert!(matches!(received, JniCommand::EnableShm));

        tx.send(JniCommand::DisableShm).unwrap();
        let received = rx.try_recv().unwrap();
        assert!(matches!(received, JniCommand::DisableShm));
    }

    #[test]
    fn crossbeam_channel_multiple_messages() {
        let (tx, rx) = crossbeam_channel::unbounded::<JniCommand>();

        for i in 0..100 {
            tx.send(JniCommand::SetScrollSensitivity { value: i as f32 }).unwrap();
        }

        for i in 0..100 {
            let received = rx.try_recv().unwrap();
            match received {
                JniCommand::SetScrollSensitivity { value } => assert_eq!(value, i as f32),
                _ => panic!("Unexpected command"),
            }
        }

        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn oneshot_channel_response() {
        let (tx, rx) = std::sync::mpsc::channel();
        let cmd = JniCommand::GetSurfaceSize { response: tx };

        let response_tx = match &cmd {
            JniCommand::GetSurfaceSize { response } => response.clone(),
            _ => unreachable!(),
        };
        response_tx.send((1920, 1080)).unwrap();

        let result = rx.recv().unwrap();
        assert_eq!(result, (1920, 1080));
    }

    #[test]
    fn native_window_ptr_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<NativeWindowPtr>();
        assert_sync::<NativeWindowPtr>();
    }

    #[test]
    fn global_channel_initialization() {
        let (tx, rx) = crossbeam_channel::unbounded::<JniCommand>();
        let prev = COMMAND_TX.get().cloned();

        set_command_tx(tx);
        assert!(send_command(JniCommand::EnableShm));

        let received = if prev.is_some() {
            true
        } else {
            rx.try_recv().is_ok()
        };
        assert!(received);
    }

    #[test]
    fn send_command_all_variants_compile_and_send() {
        let (tx, rx) = crossbeam_channel::unbounded::<JniCommand>();

        let cmds: Vec<JniCommand> = vec![
            JniCommand::TouchInput { action: 0, id: 0, x: 100.0, y: 200.0 },
            JniCommand::KeyInput { keycode: 10, is_down: true },
            JniCommand::TextInput { text: "hello".into() },
            JniCommand::RelativeMotion { dx: 10.0, dy: 20.0, time: 1000 },
            JniCommand::TrackpadClick { state: 1, button: 0x110, time: 1001 },
            JniCommand::SetScrollSensitivity { value: 0.5 },
            JniCommand::SetRefreshRate { value: 60.0 },
            JniCommand::SetResolution { width: 1920, height: 1080 },
            JniCommand::SetScale { scale: 1.5 },
            JniCommand::EnableShm,
            JniCommand::DisableShm,
            JniCommand::EnableDmabuf,
            JniCommand::SetRelativeSensitivity { value: 1.0 },
            JniCommand::SetPhysicalSize { width_mm: 155, height_mm: 87 },
            JniCommand::SuspendRendering,
            JniCommand::ResumeRendering,
            JniCommand::UpdateClipboard { text: "clip".into() },
            JniCommand::SurfaceChanged {
                width: 1920, height: 1080,
                physical_width_mm: 155, physical_height_mm: 87,
            },
        ];

        for cmd in cmds {
            tx.send(cmd).unwrap();
        }

        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        assert_eq!(count, 17);
    }

    #[test]
    fn runtime_stats_cache_default() {
        assert_eq!(get_runtime_stats(), "");
    }

    #[test]
    fn runtime_stats_cache_write_read() {
        set_runtime_stats("surfaces=3 focus=none".into());
        let stats = get_runtime_stats();
        assert!(stats.contains("surfaces=3"));
    }
}
