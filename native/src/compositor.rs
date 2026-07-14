use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::sync::mpsc;
use std::path::Path;
use std::fs;
use std::os::unix::fs::FileTypeExt;
use std::ptr;

use crate::android::backend::smithay_backend::{AndroidSmithayState, flush_deferred_composite, render_background_tick, RenderFrame};
use crate::android::backend::wayland::input::{InputRouter, RoutedInputEvent};
use crate::android::backend::wayland::seat::WinlandInputMode;
use crate::android::command_channel::{JniCommand, set_command_tx};
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::Resource;
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::data_device::set_data_device_selection;

struct CompositorRuntime {
    running: Arc<AtomicBool>,
    worker: JoinHandle<()>,
}

static RUNTIME: AtomicPtr<CompositorRuntime> = AtomicPtr::new(ptr::null_mut());

fn is_wayland_socket_published(socket_dir: &Path, socket_name: &str) -> bool {
    let socket_path = socket_dir.join(socket_name);
    match fs::metadata(&socket_path) {
        Ok(metadata) => metadata.file_type().is_socket(),
        Err(_) => false,
    }
}

pub fn spawn(distro_id: &str) -> Result<(), String> {
    let app_context = crate::android::utils::context::get_application_context();
    let data_dir = app_context.data_dir.to_string_lossy().to_string();
    let socket_dir = app_context.data_dir.join("tmp");

    // Always update XKB config for current distro, even if compositor is already running.
    crate::android::backend::wayland::smithay_runtime::configure_xkb(&data_dir, distro_id);

    if !RUNTIME.load(Ordering::SeqCst).is_null() {
        let runtime_ptr = RUNTIME.load(Ordering::SeqCst);
        let runtime = unsafe { &*runtime_ptr };
        if runtime.worker.is_finished() {
            log::warn!("Compositor: thread is dead, clearing stale runtime pointer");
            let old = RUNTIME.swap(ptr::null_mut(), Ordering::SeqCst);
            unsafe { drop(Box::from_raw(old)) };
        } else {
            log::info!("Compositor: runtime already running (xkb updated for distro={})", distro_id);
            return Ok(());
        }
    }

    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<JniCommand>();
    set_command_tx(cmd_tx);

    crate::android::backend::wayland::bind::bind_socket(socket_dir.to_string_lossy().as_ref());

    let running = Arc::new(AtomicBool::new(true));
    let thread_running = Arc::clone(&running);
    let thread_socket_dir = socket_dir.clone();
    let (startup_tx, startup_rx) = mpsc::channel::<Result<(), String>>();
    let (render_tx, render_rx) = crossbeam_channel::unbounded::<RenderFrame>();
    let (presentation_tx, presentation_rx) = crossbeam_channel::unbounded::<u64>();

    let worker = thread::spawn(move || {
        let mut wayland_server = match crate::android::backend::wayland::smithay_runtime::WaylandServer::bind(
            &thread_socket_dir,
            render_tx,
            presentation_rx,
        ) {
            Ok(server) => {
                if !is_wayland_socket_published(&thread_socket_dir, server.socket_name()) {
                    let _ = startup_tx.send(Err(format!(
                        "Compositor: bind returned success but socket is missing at {}/{}",
                        thread_socket_dir.display(),
                        server.socket_name()
                    )));
                    return;
                }
                let _ = startup_tx.send(Ok(()));
                log::info!(
                    "Compositor: published Wayland socket {} in {}",
                    server.socket_name(),
                    thread_socket_dir.display()
                );
                Some(server)
            }
            Err(error) => {
                let _ = startup_tx.send(Err(format!(
                    "Compositor: failed to publish Wayland socket in {}: {}",
                    thread_socket_dir.display(),
                    error
                )));
                log::error!(
                    "Compositor: failed to publish Wayland socket in {}: {}",
                    thread_socket_dir.display(),
                    error
                );
                return;
            }
        };

        let mut backend_state = AndroidSmithayState::new(presentation_tx);
        let mut input_router = InputRouter::default();
        let mut last_frame_tick = std::time::Instant::now();
        log::info!("Compositor: runtime loop started");

        while thread_running.load(Ordering::Relaxed) {
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    JniCommand::TouchInput { action, id, x, y } => {
                        let y_offset = backend_state.y_offset as f32;
                        let adjusted_y = if y > y_offset { y - y_offset } else { 0.0 };
                        let logical_w = backend_state.surface_size.0;
                        let logical_h = backend_state.surface_size.1;

                        if let Some(server) = wayland_server.as_mut() {
                            if server.runtime.current_input_mode == WinlandInputMode::Trackpad {
                                // Trackpad mode: bypass InputRouter entirely.
                                // Construct RoutedInputEvent directly so the trackpad
                                // anchor lifecycle (down→anchor, move→delta, up→clear)
                                // is never disrupted by InputRouter state.
                                let point = crate::android::backend::wayland::input::TouchPoint {
                                    x, y: adjusted_y,
                                    x_norm: if logical_w > 0 { (x / logical_w as f32).clamp(0.0, 1.0) } else { 0.0 },
                                    y_norm: if logical_h > 0 { (adjusted_y / logical_h as f32).clamp(0.0, 1.0) } else { 0.0 },
                                };
                                let event = match action {
                                    0 | 5 => RoutedInputEvent::TouchDown { id, point },
                                    2 => RoutedInputEvent::TouchMove { id, point },
                                    1 | 6 => RoutedInputEvent::TouchUp { id },
                                    3 => RoutedInputEvent::TouchCancel { id },
                                    _ => return,
                                };
                                server.runtime.inject_routed_event(&event);
                                crate::android::backend::wayland::seat_injector::record_injection(&event);
                            } else {
                                // Touch / Mouse mode: use InputRouter for gesture
                                // detection, multi-touch scroll, etc.
                                let events = input_router.route_touch(action, id, x, adjusted_y, logical_w, logical_h);
                                for event in &events {
                                    server.runtime.inject_routed_event(event);
                                    crate::android::backend::wayland::seat_injector::record_injection(event);
                                }
                            }
                        }
                    }
                    JniCommand::KeyInput { keycode, is_down } => {
                        let event = input_router.route_key(keycode, is_down);
                        if let Some(server) = wayland_server.as_mut() {
                            server.runtime.inject_routed_event(&event);
                            crate::android::backend::wayland::seat_injector::record_injection(&event);
                        }
                    }
                    JniCommand::TextInput { text } => {
                        let event = RoutedInputEvent::TextCommit { text };
                        if let Some(server) = wayland_server.as_mut() {
                            server.runtime.inject_routed_event(&event);
                            crate::android::backend::wayland::seat_injector::record_injection(&event);
                        }
                    }
                    JniCommand::ShutdownCompositor { response } => {
                        thread_running.store(false, Ordering::Relaxed);
                        let _ = response.send(());
                    }
                    JniCommand::SetInputMode { mode } => {
                        if let Some(server) = wayland_server.as_mut() {
                            server.runtime.set_input_mode(WinlandInputMode::from_bits(mode));
                        }
                    }
                    JniCommand::RelativeMotion { dx, dy, time } => {
                        if let Some(server) = wayland_server.as_mut() {
                            server.runtime.inject_trackpad_relative(dx, dy, time);
                        }
                    }
                    JniCommand::TrackpadClick { state, button, time } => {
                        if let Some(server) = wayland_server.as_mut() {
                            server.runtime.inject_trackpad_click(state, button, time);
                        }
                    }
                    JniCommand::SetRelativeSensitivity { value } => {
                        if let Some(server) = wayland_server.as_mut() {
                            server.runtime.relative_sensitivity = value;
                        }
                    }
                    JniCommand::SurfaceChanged { width, height, physical_width_mm, physical_height_mm } => {
                        // If the user has requested a specific resolution (via Dashboard),
                        // persist it through Android lifecycle surface recreations.
                        // Otherwise, use the native dimensions from surfaceChanged.
                        let effective_size = backend_state.requested_resolution.unwrap_or((width, height));
                        backend_state.surface_size = effective_size;
                        if physical_width_mm > 0 && physical_height_mm > 0 {
                            backend_state.physical_size_mm = (physical_width_mm.max(1), physical_height_mm.max(1));
                        }
                        if effective_size.0 > 0 && effective_size.1 > 0 {
                            // Re-apply user-requested scale if set (survives lifecycle).
                            if let Some(saved_scale) = backend_state.requested_scale {
                                crate::android::command_channel::set_scale(saved_scale);
                            }
                            let scale = crate::android::command_channel::get_scale();
                            backend_state.current_scale = scale;
                            let logical_w = (effective_size.0 as f32 / scale).round() as i32;
                            let logical_h = (effective_size.1 as f32 / scale).round() as i32;
                            if let Some(server) = wayland_server.as_mut() {
                                server.runtime.update_output_mode(effective_size.0, effective_size.1, None);
                            }
                            crate::android::command_channel::set_logical_size(logical_w.max(1), logical_h.max(1));
                        } else {
                            crate::android::command_channel::set_logical_size(effective_size.0.max(1), effective_size.1.max(1));
                        }
                        // Update caches immediately so seat.rs reads current values
                        crate::android::command_channel::set_surface_size(backend_state.surface_size.0, backend_state.surface_size.1);
                        crate::android::command_channel::set_physical_size(backend_state.physical_size_mm.0, backend_state.physical_size_mm.1);
                    }
                    JniCommand::BindNativeWindow { native_window, response } => {
                        let ptr = native_window.0 as *mut ndk_sys::ANativeWindow;
                        let result = crate::android::backend::smithay_backend::bind_native_window(&mut backend_state, ptr);
                        if result.is_ok() {
                            if let Some(server) = wayland_server.as_mut() {
                                server.runtime.apply_forced_focus("resume");
                                server.runtime.render_all();
                            }
                        }
                        let _ = response.send(result);
                    }
                    JniCommand::ReleaseNativeWindow { response } => {
                        crate::android::backend::smithay_backend::release_native_window(&mut backend_state);
                        let _ = response.send(());
                    }
                    JniCommand::SetScrollSensitivity { value } => {
                        backend_state.scroll_sensitivity = value.clamp(0.1, 5.0);
                    }
                    JniCommand::SetRefreshRate { value } => {
                        backend_state.refresh_rate = value.clamp(30.0, 240.0);
                    }
                    JniCommand::SetPhysicalSize { width_mm, height_mm } => {
                        backend_state.physical_size_mm = (width_mm.max(1), height_mm.max(1));
                    }
                    JniCommand::SetYOffset { y_offset } => {
                        backend_state.y_offset = y_offset.max(0);
                    }
                    JniCommand::EnableShm => {
                        crate::android::backend::smithay_backend::enable_shm_on_compositor(&mut backend_state);
                    }
                    JniCommand::DisableShm => {
                        backend_state.shm_enabled = false;
                        backend_state.shm_region = None;
                    }
                    JniCommand::EnableDmabuf => {
                        log::info!("Compositor: DMA-BUF not supported (SHM-only mode)");
                    }
                    JniCommand::SuspendRendering => {
                        crate::android::backend::smithay_backend::suspend_native_window(&mut backend_state);
                        if let Some(server) = wayland_server.as_mut() {
                            server.runtime.clear_focused_surface();
                            server.runtime.clear_input_state();
                        }
                        input_router.clear();
                    }
                    JniCommand::ResumeRendering => {}
                    JniCommand::SetResolution { width, height } => {
                        backend_state.surface_size = (width, height);
                        backend_state.requested_resolution = Some((width, height));
                        crate::android::command_channel::set_surface_size(width, height);
                    }
                    JniCommand::SetScale { scale } => {
                        backend_state.requested_scale = Some(scale);
                        backend_state.current_scale = scale;
                        crate::android::command_channel::set_scale(scale);
                        let (w, h) = backend_state.surface_size;
                        if w > 0 && h > 0 {
                            let logical_w = (w as f32 / scale).round() as i32;
                            let logical_h = (h as f32 / scale).round() as i32;
                            crate::android::command_channel::set_logical_size(logical_w.max(1), logical_h.max(1));
                            if let Some(server) = wayland_server.as_mut() {
                                server.runtime.update_output_mode(w, h, None);
                            }
                        }
                    }
                    JniCommand::UpdateClipboard { text } => {
                        log::info!("[DD_DC] UpdateClipboard handler: re-setting clipboard as Compositor source len={}", text.len());
                        if let Some(server) = wayland_server.as_mut() {
                            if let Ok(mut guard) = server.runtime.clipboard_text.lock() {
                                *guard = text.clone();
                            }
                            set_data_device_selection::<crate::android::backend::wayland::seat::AndroidSeatRuntime>(
                                &server.runtime.display_handle,
                                &server.runtime.seat,
                                vec![
                                    "text/plain;charset=utf-8".to_string(),
                                    "text/plain".to_string(),
                                ],
                                text,
                            );
                        }
                    }
                    JniCommand::WaylandClipboardToAndroid { text } => {
                        crate::android::bridge_clipboard::set_clipboard_text(&text);
                    }
                    JniCommand::GetRuntimeStats { response } => {
                        let stats = crate::android::command_channel::get_runtime_stats();
                        let _ = response.send(stats);
                    }
                    JniCommand::GetSurfaceSize { response } => {
                        let _ = response.send(backend_state.surface_size);
                    }
                    JniCommand::GetScrollSensitivity { response } => {
                        let _ = response.send(backend_state.scroll_sensitivity);
                    }
                    JniCommand::GetBackendSnapshot { response } => {
                        let (sw, sh) = backend_state.surface_size;
                        let snap = format!(
                            "window_bound={}, surface={}x{}, wl_scale={:.1}, shm_enabled={}",
                            backend_state.native_window.is_some(),
                            sw, sh,
                            crate::android::command_channel::get_scale(),
                            backend_state.shm_enabled,
                        );
                        let _ = response.send(snap);
                    }
                }
            }

            if let Some(server) = wayland_server.as_mut() {
                // Check gesture timer (long-press detection)
                if let Some(event) = input_router.poll_timer() {
                    server.runtime.inject_routed_event(&event);
                    crate::android::backend::wayland::seat_injector::record_injection(&event);
                }

                server.pump();
                let connected = server.connected_client_count() > 0;
                crate::android::command_channel::set_clients_connected(connected);
                if connected {
                    let frame_interval = Duration::from_secs_f32(
                        1.0 / backend_state.refresh_rate.clamp(30.0, 240.0),
                    );
                    if last_frame_tick.elapsed() >= frame_interval {
                        server.runtime.frame_tick();
                        last_frame_tick = std::time::Instant::now();
                    }
                }
            }
            flush_deferred_composite(&mut backend_state, &render_rx);
            render_background_tick(&backend_state);

            // Update backend state caches for JNI diagnostics
            let (lw, lh) = backend_state.surface_size;
            let cached_scale = crate::android::command_channel::get_scale();
            let snapshot = format!(
                "window_bound={}, surface={}x{}, wl_scale={:.1}, shm_enabled={}",
                backend_state.native_window.is_some(),
                lw, lh,
                cached_scale,
                backend_state.shm_enabled,
            );
            crate::android::command_channel::set_backend_snapshot(snapshot);
            crate::android::command_channel::set_surface_size(backend_state.surface_size.0, backend_state.surface_size.1);
            let logical_w = (backend_state.surface_size.0 as f32 / cached_scale).round() as i32;
            let logical_h = (backend_state.surface_size.1 as f32 / cached_scale).round() as i32;
            crate::android::command_channel::set_logical_size(logical_w.max(1), logical_h.max(1));
            crate::android::command_channel::set_scroll_sensitivity(backend_state.scroll_sensitivity);
            crate::android::command_channel::set_physical_size(backend_state.physical_size_mm.0, backend_state.physical_size_mm.1);

            if let Some(server) = wayland_server.as_ref() {
                let focused = server.runtime.focused_surface.as_ref()
                    .map(|s| format!("{:?}", s.id()))
                    .unwrap_or_else(|| "none".to_string());
                let stats = format!(
                    "windows={} unmanaged={} focus={} injected={} dispatch={} focus_decision={} cursor={}",
                    server.runtime.space.elements().count(),
                    server.runtime.unmanaged_surfaces.len(),
                    focused,
                    server.runtime.injected_events,
                    server.runtime.last_seat_dispatch,
                    server.runtime.last_focus_decision,
                    server.runtime.last_cursor_mode
                );
                crate::android::command_channel::set_runtime_stats(stats);
            }

            // Keep pumping connected clients without busy-spinning. Rendering is
            // driven by surface commits/input/output changes, not every poll.
            if wayland_server.as_mut().is_some_and(|s| s.connected_client_count() > 0) {
                thread::sleep(Duration::from_millis(1));
            } else {
                thread::sleep(Duration::from_millis(8));
            }
        }
        log::info!("Compositor: runtime loop stopped");
    });

    match startup_rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            running.store(false, Ordering::Relaxed);
            let _ = worker.join();
            return Err(error);
        }
        Err(error) => {
            running.store(false, Ordering::Relaxed);
            let _ = worker.join();
            return Err(format!(
                "Compositor startup timed out before publishing socket: {error}"
            ));
        }
    }

    log::info!("Compositor: initialized");
    RUNTIME.store(Box::into_raw(Box::new(CompositorRuntime { running, worker })), Ordering::SeqCst);
    Ok(())
}

pub fn stop() {
    let runtime_ptr = RUNTIME.swap(ptr::null_mut(), Ordering::SeqCst);
    if runtime_ptr.is_null() {
        return;
    }

    let runtime = unsafe { Box::from_raw(runtime_ptr) };

    let (tx, rx) = mpsc::channel();
    crate::android::command_channel::send_command(JniCommand::ShutdownCompositor { response: tx });
    let _ = rx.recv_timeout(Duration::from_secs(5));

    runtime.running.store(false, Ordering::Relaxed);
    let _ = runtime.worker.join();
    log::info!("Compositor: stopped and cleaned up");
}

#[cfg(all(test, feature = "smithay_android"))]
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::ptr;
    use std::sync::atomic::{AtomicBool, Ordering, AtomicPtr};
    use super::CompositorRuntime;

    // Helper: create a clean AtomicPtr not shared with other tests
    fn fresh_runtime_ptr() -> AtomicPtr<CompositorRuntime> {
        AtomicPtr::new(ptr::null_mut())
    }

    #[test]
    fn starts_null() {
        let rt = fresh_runtime_ptr();
        assert!(rt.load(Ordering::SeqCst).is_null());
    }

    #[test]
    fn store_and_load() {
        let rt = fresh_runtime_ptr();
        let dummy = Box::into_raw(Box::new(CompositorRuntime {
            running: Arc::new(AtomicBool::new(true)),
            worker: thread::spawn(|| {}),
        }));

        rt.store(dummy, Ordering::SeqCst);
        assert!(!rt.load(Ordering::SeqCst).is_null());

        let taken = rt.swap(ptr::null_mut(), Ordering::SeqCst);
        assert!(!taken.is_null());
        assert!(rt.load(Ordering::SeqCst).is_null());

        let recovered = unsafe { Box::from_raw(taken) };
        recovered.running.store(false, Ordering::Relaxed);
        let _ = recovered.worker.join();
    }

    #[test]
    fn swap_returns_old_value() {
        let rt = fresh_runtime_ptr();

        let dummy1 = Box::into_raw(Box::new(CompositorRuntime {
            running: Arc::new(AtomicBool::new(true)),
            worker: thread::spawn(|| {}),
        }));
        rt.store(dummy1, Ordering::SeqCst);

        let old = rt.swap(ptr::null_mut(), Ordering::SeqCst);
        assert_eq!(old, dummy1);

        let recovered = unsafe { Box::from_raw(old) };
        recovered.running.store(false, Ordering::Relaxed);
        let _ = recovered.worker.join();
    }

    #[test]
    fn double_swap_is_safe() {
        let rt = fresh_runtime_ptr();
        assert!(rt.swap(ptr::null_mut(), Ordering::SeqCst).is_null());
        assert!(rt.swap(ptr::null_mut(), Ordering::SeqCst).is_null());
    }

    #[test]
    fn null_check_after_swap() {
        let rt = fresh_runtime_ptr();
        let dummy = Box::into_raw(Box::new(CompositorRuntime {
            running: Arc::new(AtomicBool::new(true)),
            worker: thread::spawn(|| {}),
        }));

        rt.store(dummy, Ordering::SeqCst);
        assert!(!rt.load(Ordering::SeqCst).is_null());

        rt.swap(ptr::null_mut(), Ordering::SeqCst);
        assert!(rt.load(Ordering::SeqCst).is_null());

        // Clean up
        let recovered = unsafe { Box::from_raw(dummy) };
        recovered.running.store(false, Ordering::Relaxed);
        let _ = recovered.worker.join();
    }
}
