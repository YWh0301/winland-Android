use crate::android::backend::wayland::engine_timing;
#[cfg(feature = "smithay_android")]
use std::fs;
#[cfg(feature = "smithay_android")]
use std::os::unix::fs::FileTypeExt;
#[cfg(feature = "smithay_android")]
use std::os::unix::net::UnixStream;
#[cfg(feature = "smithay_android")]
use std::panic::{catch_unwind, AssertUnwindSafe};
#[cfg(feature = "smithay_android")]
use std::path::{Path, PathBuf};
#[cfg(feature = "smithay_android")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(feature = "smithay_android")]
use std::sync::Arc;
#[cfg(feature = "smithay_android")]
use std::time::Duration;
#[cfg(feature = "smithay_android")]
use smithay::reexports::calloop;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::{Display, ListeningSocket};
#[cfg(feature = "smithay_android")]
use smithay::wayland::compositor::CompositorClientState;
#[cfg(feature = "smithay_android")]
use smithay::xwayland::X11Wm;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::seat::AndroidSeatRuntime;

/// Maximum number of XWayland reconnection attempts (~10s at 500ms intervals).
#[cfg(feature = "smithay_android")]
const XWAYLAND_MAX_RECONNECT_ATTEMPTS: u32 = 20;

// ── WaylandClientState ───────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
#[derive(Default)]
pub(crate) struct WaylandClientState {
    pub(crate) compositor_state: CompositorClientState,
}

#[cfg(feature = "smithay_android")]
impl ClientData for WaylandClientState {
    fn initialized(&self, client_id: ClientId) {
        log::info!(
            "SmithayRuntime: wayland client initialized id={:?}",
            client_id
        );
    }

    fn disconnected(&self, client_id: ClientId, reason: DisconnectReason) {
        log::info!(
            "SmithayRuntime: wayland client disconnected id={:?} reason={:?}",
            client_id,
            reason
        );
    }
}

// ── xkb_path_accessible ──────────────────────────────────────────────────────
// SELinux (untrusted_app) may block stat() on app-data paths, causing
// Path::exists() to return false even though the path is valid.
// This helper treats PermissionDenied as "accessible" and logs a warning.

fn xkb_path_accessible(path: &str) -> bool {
    match std::path::Path::new(path).try_exists() {
        Ok(true) => true,
        Ok(false) => false,
        Err(e) => {
            log::warn!(
                "XKB: Permission check blocked for '{}' ({}). Assuming path exists.",
                path, e
            );
            true
        }
    }
}

// ── configure_xkb ────────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
pub fn configure_xkb(data_dir: &str, distro_id: &str) {
    let xkb_path = format!("{}/rootfs_{}/usr/share/X11/xkb", data_dir, distro_id);

    if xkb_path_accessible(&xkb_path) {
        let rules_path = format!("{}/rules/evdev", xkb_path);
        match std::fs::File::open(&rules_path) {
            Ok(_) => {
                std::env::set_var("XKB_CONFIG_ROOT", &xkb_path);
                log::info!("XKB: Configured XKB_CONFIG_ROOT={}", xkb_path);
                return;
            }
            Err(e) => {
                log::warn!(
                    "XKB: main path blocked or rules/evdev missing at '{}': {}. Trying fallback.",
                    rules_path, e
                );
            }
        }
    }

    // Fallback: xkeyboard-config (some distros install xkb data here)
    let share_dir = format!("{}/rootfs_{}/usr/share", data_dir, distro_id);
    if let Ok(share_entries) = std::fs::read_dir(&share_dir) {
        for entry in share_entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("xkeyboard-config") {
                let xkc_path = entry.path().display().to_string();
                if xkb_path_accessible(&xkc_path) {
                    log::info!(
                        "XKB: X11/xkb inaccessible; using {} instead.",
                        name_str
                    );
                    std::env::set_var("XKB_CONFIG_ROOT", &xkc_path);
                    return;
                }
            }
        }
    }

    // Fallback: try other installed distros
    let data_dir_path = std::path::Path::new(data_dir);
    if let Ok(entries) = std::fs::read_dir(data_dir_path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(rest) = name_str.strip_prefix("rootfs_") {
                let alt_path = format!("{}/usr/share/X11/xkb", entry.path().display());
                if xkb_path_accessible(&alt_path) {
                    log::info!(
                        "XKB: distro '{}' path missing; using fallback from '{}'",
                        distro_id, rest
                    );
                    std::env::set_var("XKB_CONFIG_ROOT", &alt_path);
                    return;
                }
            }
        }
    }

    // Last resort: check system paths
    for alt in &["/system/usr/share/X11/xkb", "/usr/share/X11/xkb"] {
        if xkb_path_accessible(alt) {
            log::warn!("XKB: No rootfs xkb found; using system fallback at {}", alt);
            std::env::set_var("XKB_CONFIG_ROOT", alt);
            return;
        }
    }

    log::error!(
        "XKB: Path {} does not exist and no fallback found.",
        xkb_path
    );
}

// ── WaylandServer ────────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
pub struct WaylandServer {
    pub(crate) runtime: AndroidSeatRuntime,
    display: Display<AndroidSeatRuntime>,
    listener: ListeningSocket,
    socket_name: String,
    socket_path: PathBuf,
    accepted_clients: u64,
    missing_socket_reported: bool,
    xwayland_event_loop: Option<calloop::EventLoop<'static, AndroidSeatRuntime>>,
    xwayland_reconnect_at: Option<std::time::Instant>,
    xwayland_reconnect_attempts: u32,
    xwayland_watch_started: bool,
    xwayland_connect_trigger: Arc<AtomicBool>,
}

#[cfg(feature = "smithay_android")]
impl WaylandServer {
    pub(crate) fn bind(
        socket_dir: &Path,
        render_sender: crossbeam_channel::Sender<crate::android::backend::smithay_backend::RenderFrame>,
    ) -> Result<Self, String> {
        std::env::set_var("XDG_RUNTIME_DIR", socket_dir);
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");

        let display = Display::<AndroidSeatRuntime>::new()
            .map_err(|error| format!("failed to create wayland display: {error}"))?;

        // Always use logical_size for wl_output.mode — never surface_size.
        // surface_size is the GLES viewport (may be 720p) and must be invisible
        // to Wayland clients so they always draw at the safe native resolution.
        let (width, height) = crate::android::command_channel::get_logical_size();
        let (final_w, final_h) = if width > 0 && height > 0 {
            (width, height)
        } else {
            (1080, 1920)
        };

        let runtime = AndroidSeatRuntime::new(&display.handle(), final_w, final_h, render_sender)
            .map_err(|error| format!("failed to initialize Smithay runtime: {error}"))?;

        let socket_path = socket_dir.join("wayland-0");
        let lock_path = socket_dir.join("wayland-0.lock");

        for stale in [&lock_path, &socket_path] {
            match fs::remove_file(stale) {
                Ok(_) => {
                    log::info!("SmithayRuntime: removed stale {}", stale.display());
                    engine_timing::append_runtime_trace(
                        socket_dir,
                        &format!("bind: removed stale {}", stale.display()),
                    );
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    log::warn!(
                        "SmithayRuntime: failed to remove stale {}: {e}",
                        stale.display()
                    );
                    engine_timing::append_runtime_trace(
                        socket_dir,
                        &format!("bind: WARN could not remove {}: {e}", stale.display()),
                    );
                }
            }
        }

        if socket_dir.exists() && !socket_dir.is_dir() {
            fs::remove_file(socket_dir)
                .map_err(|e| format!("socket_dir {} is not a directory and could not be removed: {e}", socket_dir.display()))?;
        }
        fs::create_dir_all(socket_dir)
            .map_err(|e| format!("failed to create socket_dir {}: {e}", socket_dir.display()))?;

        let test_path = socket_dir.join(".bind-test");
        match fs::File::create(&test_path) {
            Ok(_) => {
                let _ = fs::remove_file(&test_path);
                log::info!("SmithayRuntime: socket_dir is writable");
            }
            Err(e) => {
                log::error!("SmithayRuntime: socket_dir NOT writable: {e}");
                engine_timing::append_runtime_trace(
                    socket_dir,
                    &format!("bind: FATAL socket_dir not writable: {e}"),
                );
                return Err(format!(
                    "socket_dir {} not writable: {e}",
                    socket_dir.display()
                ));
            }
        }

        log::info!(
            "WinlandV2: Final binding path: {}/wayland-0",
            socket_dir.display()
        );
        let listener = ListeningSocket::bind_absolute(socket_path.clone())
            .map_err(|error| format!("failed to bind {}: {error}", socket_path.display()))?;

        log::info!(
            "--- [Winland-Alpha-Final-V3] --- Wayland socket bound at {:?}. Applying 0666 permissions via libc::chmod loop.",
            socket_path
        );

        let c_socket_path = std::ffi::CString::new(socket_path.to_string_lossy().as_bytes()).unwrap();
        let mut success = false;

        for attempt in 1..=5 {
            std::thread::sleep(std::time::Duration::from_millis(150));
            unsafe {
                if libc::chmod(c_socket_path.as_ptr(), 0o666) == 0 {
                    log::info!("Wayland: Socket permissions set to 0666 successfully on attempt {}", attempt);
                    success = true;
                    break;
                }
            }
            log::warn!("Wayland: chmod attempt {} failed for {:?}", attempt, socket_path);
        }

        if !success {
            log::error!("Wayland: FATAL - Failed to set socket permissions after 5 attempts.");
        }

        let socket_is_socket = fs::symlink_metadata(&socket_path)
            .map(|metadata| metadata.file_type().is_socket())
            .unwrap_or(false);
        engine_timing::append_runtime_trace(
            socket_dir,
            &format!(
                "bind: success path={} is_socket={} perms_updated=true",
                socket_path.display(),
                socket_is_socket
            ),
        );

        log::info!(
            "SmithayRuntime: listening on {}/wayland-0",
            socket_dir.display()
        );

        log::info!(
            "SmithayRuntime: WAYLAND_DISPLAY=wayland-0 XDG_RUNTIME_DIR={}",
            socket_dir.display()
        );

        log::info!("SmithayRuntime: XWayland protocol state active on display (passive host)");

        Ok(Self {
            runtime,
            display,
            listener,
            socket_name: "wayland-0".to_string(),
            socket_path,
            accepted_clients: 0,
            missing_socket_reported: false,
            xwayland_event_loop: None,
            xwayland_reconnect_at: None,
            xwayland_reconnect_attempts: 0,
            xwayland_watch_started: false,
            xwayland_connect_trigger: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn socket_name(&self) -> &str {
        &self.socket_name
    }

    pub fn start_xwayland_watcher(&mut self, display_num: i32) {
        if self.xwayland_watch_started {
            return;
        }
        self.xwayland_watch_started = true;

        let base_dir = crate::android::command_channel::get_x11_socket_dir();
        if base_dir.is_empty() {
            log::warn!("XWayland: cannot start watcher, no socket dir set");
            return;
        }

        let dir_path = format!("{}/.X11-unix", base_dir);
        let x_socket_path = format!("{}/X{}", dir_path, display_num);

        // Immediate check: X0 might already exist (race-proof before watcher setup).
        if std::path::Path::new(&x_socket_path).exists() {
            log::info!("XWayland: X{} socket already exists, connecting directly", display_num);
            self.connect_xwayland(display_num);
            return;
        }

        log::info!("XWayland: starting inotify watcher on {}", dir_path);

        let inotify = match nix::sys::inotify::Inotify::init(nix::sys::inotify::InitFlags::empty()) {
            Ok(ino) => ino,
            Err(e) => {
                log::error!("XWayland: inotify init failed: {}", e);
                return;
            }
        };

        if let Err(e) = inotify.add_watch(dir_path.as_str(), nix::sys::inotify::AddWatchFlags::IN_CREATE) {
            log::error!("XWayland: inotify add_watch on {} failed: {}", dir_path, e);
            return;
        }

        let trigger = self.xwayland_connect_trigger.clone();
        let target = format!("X{}", display_num);

        std::thread::Builder::new()
            .name("xwayland-watcher".into())
            .spawn(move || {
                loop {
                    match inotify.read_events() {
                        Ok(events) => {
                            for event in &events {
                                if let Some(ref name) = event.name {
                                    if name.to_string_lossy() == target {
                                        log::info!("XWayland: inotify detected {} creation", target);
                                        trigger.store(true, Ordering::SeqCst);
                                        return;
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("XWayland: inotify read error: {}", e);
                            return;
                        }
                    }
                }
            })
            .ok();
    }

    pub fn connect_xwayland(&mut self, display_num: i32) {
        if self.xwayland_event_loop.is_some() {
            return;
        }
        let base_dir = crate::android::command_channel::get_x11_socket_dir();
        let socket_path = if base_dir.is_empty() {
            format!("/tmp/.X11-unix/X{}", display_num)
        } else {
            format!("{}/.X11-unix/X{}", base_dir, display_num)
        };
        let stream = match UnixStream::connect(&socket_path) {
            Ok(s) => s,
            Err(e) => {
                log::error!("XWayland: cannot connect to {}: {}", socket_path, e);
                return;
            }
        };
        // Retry xwayland_client with backoff — there's a tiny race between
        // XWayland creating the X11 socket and registering as a Wayland client.
        let (client, dh) = loop {
            if let Some(c) = self.runtime.xwayland_client.clone() {
                break (c, self.runtime.display_handle.clone());
            }
            log::warn!("XWayland: xwayland_client not ready, short retry...");
            std::thread::sleep(Duration::from_millis(100));
            if self.runtime.xwayland_client.is_none() {
                // After ~300ms total, the race is almost certainly a real failure.
                // Wait one more beat then give up (reconnect timer handles retry).
                std::thread::sleep(Duration::from_millis(200));
            }
            match self.runtime.xwayland_client.clone() {
                Some(c) => break (c, self.runtime.display_handle.clone()),
                None => {
                    log::warn!("XWayland: xwayland_client not ready after retries, giving up");
                    return;
                }
            }
        };
        let event_loop: calloop::EventLoop<'static, AndroidSeatRuntime> =
            match calloop::EventLoop::try_new() {
                Ok(el) => el,
                Err(e) => {
                    log::error!("XWayland: failed to create EventLoop: {}", e);
                    return;
                }
            };
        let handle = event_loop.handle();
        let mut wm = match X11Wm::start_wm(handle, &dh, stream, client) {
             Ok(wm) => wm,
             Err(e) => {
                 log::error!("XWayland: X11Wm::start_wm failed: {}", e);
                 return;
             }
         };
         // Hide the X11 root cursor — our software cursor does all rendering.
         // Prevents a double-cursor artifact where XWayland's glyph cursor
         // shows on top of the compositor's software cursor.
         {
             let transparent = vec![0u8; 4]; // 1×1 RGBA fully transparent
             let _ = wm.set_cursor(&transparent, (1u16, 1u16).into(), (0u16, 0u16).into());
         }
         let xwm_id = wm.id();
         self.runtime.x11_wm = Some(wm);
        self.runtime.xwayland_shell_state.xwm_id = Some(xwm_id);
        self.xwayland_event_loop = Some(event_loop);
        log::info!("XWayland: connected to display :{}", display_num);
    }

    pub fn disconnect_all_clients(&mut self) {
        let handle = self.display.backend().handle();
        let ids: Vec<ClientId> = {
            let mut ids = Vec::new();
            handle.with_all_clients(|id| ids.push(id));
            ids
        };
        if !ids.is_empty() {
            log::info!("SmithayRuntime: disconnecting {} client(s)", ids.len());
            for id in &ids {
                handle.kill_client(id.clone(), DisconnectReason::ConnectionClosed);
            }
        }
    }

    pub fn connected_client_count(&mut self) -> usize {
        let handle = self.display.backend().handle();
        let mut count = 0;
        handle.with_all_clients(|_| count += 1);
        count
    }

    pub fn pump(&mut self) {
        let socket_present = fs::symlink_metadata(&self.socket_path)
            .map(|metadata| metadata.file_type().is_socket())
            .unwrap_or(false);
        if !socket_present && !self.missing_socket_reported {
            self.missing_socket_reported = true;
            let socket_dir = self
                .socket_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let message = format!(
                "pump: socket missing while server alive path={}",
                self.socket_path.display()
            );
            log::error!("SmithayRuntime: {}", message);
            engine_timing::append_runtime_trace(&socket_dir, &message);
        } else if socket_present && self.missing_socket_reported {
            self.missing_socket_reported = false;
            let socket_dir = self
                .socket_path
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| PathBuf::from("."));
            let message = format!("pump: socket restored path={}", self.socket_path.display());
            log::info!("SmithayRuntime: {}", message);
            engine_timing::append_runtime_trace(&socket_dir, &message);
        }

        loop {
            let stream = match self.listener.accept() {
                Ok(stream) => stream,
                Err(error) => {
                    log::warn!("SmithayRuntime: failed while accepting clients: {}", error);
                    break;
                }
            };

            let Some(stream) = stream else {
                break;
            };

            match self
                .display
                .handle()
                .insert_client(stream, Arc::new(WaylandClientState::default()))
            {
                Ok(_) => {
                    self.accepted_clients += 1;
                    log::info!(
                        "SmithayRuntime: accepted wayland client count={}",
                        self.accepted_clients
                    );
                }
                Err(error) => {
                    log::warn!("SmithayRuntime: failed to add wayland client: {}", error);
                }
            }
        }

        if self.xwayland_event_loop.is_none() {
            let display = crate::android::command_channel::get_xwayland_display();
            if display >= 0 {
                // Start the inotify watcher on first notification from Kotlin.
                // On subsequent frames the watcher is either already running or
                // connect_xwayland succeeded (xwayland_event_loop is Some).
                if !self.xwayland_watch_started {
                    self.start_xwayland_watcher(display);
                }
                // If inotify triggered (X0 created), attempt connection once.
                if self.xwayland_connect_trigger.load(Ordering::Acquire) {
                    self.xwayland_connect_trigger.store(false, Ordering::Release);
                    self.connect_xwayland(display);
                }
            }
        }

        if let Some(ref mut event_loop) = self.xwayland_event_loop {
            if let Err(e) = event_loop.dispatch(Duration::ZERO, &mut self.runtime) {
                log::error!("XWayland: event loop dispatch error: {:?}. Will retry in 2s.", e);
                self.xwayland_event_loop = None;
                self.runtime.x11_wm = None;
                self.runtime.xwayland_shell_state.xwm_id = None;
                self.xwayland_reconnect_at = Some(std::time::Instant::now() + Duration::from_secs(2));
            }
        }

        if let Some(reconnect_at) = self.xwayland_reconnect_at {
            if std::time::Instant::now() >= reconnect_at {
                self.xwayland_reconnect_at = None;
                let display = crate::android::command_channel::get_xwayland_display();
                if display < 0 {
                    log::warn!(
                        "XWayland: reconnect attempt {}/{} skipped — display not yet set",
                        self.xwayland_reconnect_attempts + 1,
                        XWAYLAND_MAX_RECONNECT_ATTEMPTS,
                    );
                } else {
                    self.xwayland_reconnect_attempts += 1;
                    if self.xwayland_reconnect_attempts > XWAYLAND_MAX_RECONNECT_ATTEMPTS {
                        log::error!(
                            "XWayland: giving up after {} reconnection attempts",
                            XWAYLAND_MAX_RECONNECT_ATTEMPTS,
                        );
                        self.xwayland_reconnect_at = None;
                    } else {
                        log::info!(
                            "XWayland: reconnect attempt {}/{} to display :{}",
                            self.xwayland_reconnect_attempts,
                            XWAYLAND_MAX_RECONNECT_ATTEMPTS,
                            display,
                        );
                        self.connect_xwayland(display);
                        if self.xwayland_event_loop.is_some() {
                            self.xwayland_reconnect_attempts = 0;
                        }
                    }
                }
            }
        }

        let pending_pings = self.runtime.x11_pending_pings.len();
        if pending_pings > 0 && pending_pings % 100 == 0 {
            log::warn!("XWayland: {} pending pings not acked", pending_pings);
        }

        let dispatch_result = catch_unwind(AssertUnwindSafe(|| {
            self.display.dispatch_clients(&mut self.runtime)
        }));
        match dispatch_result {
            Ok(Ok(_count)) => {}
            Ok(Err(error)) => {
                log::warn!("SmithayRuntime: dispatch_clients failed: {}", error);
            }
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown cause".to_string()
                };
                log::error!(
                    "SmithayRuntime: dispatch_clients PANICKED: {}. Server recovered.",
                    msg
                );
                let _ = self.display.flush_clients();
            }
        }

        let flush_result = catch_unwind(AssertUnwindSafe(|| {
            self.display.flush_clients()
        }));
        match flush_result {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                log::warn!("SmithayRuntime: flush_clients failed: {}", error);
            }
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown cause".to_string()
                };
                log::error!(
                    "SmithayRuntime: flush_clients PANICKED: {}. Server recovered.",
                    msg
                );
            }
        }
    }
}

#[cfg(feature = "smithay_android")]
impl WaylandServer {
    pub fn flush(&mut self) -> Result<(), ()> {
        self.display.flush_clients().map_err(|_| ())
    }
}

#[cfg(feature = "smithay_android")]
impl Drop for WaylandServer {
    fn drop(&mut self) {
        let socket_dir = self
            .socket_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let message = format!(
            "drop: WaylandServer dropped path={} accepted_clients={}",
            self.socket_path.display(),
            self.accepted_clients
        );
        log::warn!("SmithayRuntime: {}", message);
        engine_timing::append_runtime_trace(&socket_dir, &message);
    }
}
