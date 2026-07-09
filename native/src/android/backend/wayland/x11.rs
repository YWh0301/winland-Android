#[cfg(feature = "smithay_android")]
use std::io::Read;
#[cfg(feature = "smithay_android")]
use smithay::desktop::Window;
#[cfg(feature = "smithay_android")]
use smithay::output::Mode as OutputMode;
#[cfg(feature = "smithay_android")]
use smithay::utils::SERIAL_COUNTER;
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::SelectionTarget;
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::data_device::{
    request_data_device_client_selection, set_data_device_selection,
};
#[cfg(feature = "smithay_android")]
use smithay::wayland::xwayland_shell::{XWaylandShellHandler, XWaylandShellState};
#[cfg(feature = "smithay_android")]
use smithay::xwayland::X11Wm;
#[cfg(feature = "smithay_android")]
use smithay::xwayland::XwmHandler;
#[cfg(feature = "smithay_android")]
use smithay::xwayland::xwm::{WmWindowType, XwmId, ResizeEdge, Reorder, WmWindowProperty, X11Surface};
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::seat::AndroidSeatRuntime;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::server::WaylandClientState;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::shell::WindowElement;

#[cfg(feature = "smithay_android")]
impl XWaylandShellHandler for AndroidSeatRuntime {
    fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
        &mut self.xwayland_shell_state
    }

    fn surface_associated(
        &mut self,
        _xwm: XwmId,
        wl_surface: smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
        _window: X11Surface,
    ) {
        use smithay::reexports::wayland_server::Resource;
        if self.xwayland_client.is_none() {
            if let Some(client) = wl_surface.client() {
                self.xwayland_client = Some(client.clone());

                if let Some(data) = client.get_data::<WaylandClientState>() {
                    data.compositor_state.set_client_scale(1.0);
                    log::info!("XWayland: set client_scale=1.0");
                } else {
                    log::info!("XWayland: WaylandClientState not found (XWaylandClientData handles compositor_state), scale stays 1.0 default");
                }
            }
        }
        log::debug!("XWayland: X11 surface associated with wl_surface");
    }
}

#[cfg(feature = "smithay_android")]
impl XwmHandler for AndroidSeatRuntime {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.x11_wm.as_mut().expect(
            "xwm_state() called before x11_wm was initialized — call activate_passive_xwayland() first"
        )
    }

    fn new_window(&mut self, _xwm: XwmId, window: X11Surface) {
        log::debug!("XWayland: new_window id={}", window.window_id());
        let window_type = window.window_type();
        let (sw, sh) = self.usable_screen_size();
        let default_w = (sw as f32 * 0.8) as i32;
        let default_h = (sh as f32 * 0.8) as i32;

        let (fx, fy, fw, fh) = match window_type {
            Some(WmWindowType::Dialog) => {
                let d_w = (sw as f32 * 0.5) as i32;
                let d_h = (sh as f32 * 0.4) as i32;
                ((sw - d_w) / 2, (sh - d_h) / 2, d_w, d_h)
            }
            Some(WmWindowType::Dock) => {
                let d_w = sw;
                let d_h = (sh as f32 * 0.08) as i32;
                (0, self.reserved_top, d_w, d_h)
            }
            Some(WmWindowType::Splash) => {
                let s_w = (sw as f32 * 0.5) as i32;
                let s_h = (sh as f32 * 0.4) as i32;
                ((sw - s_w) / 2, (sh - s_h) / 2, s_w, s_h)
            }
            _ => {
                let offset_count = self.wl_to_window.len() as i32;
                let cascade = 30;
                let base_x = (sw - default_w) / 2;
                let base_y = self.reserved_top + 40;
                let fx = base_x + (offset_count % 10) * cascade;
                let fy = base_y + (offset_count % 10) * cascade;
                (fx.max(0), fy.max(self.reserved_top), default_w, default_h)
            }
        };
        let rect = smithay::utils::Rectangle::new(
            (fx, fy).into(),
            (fw.max(100), fh.max(100)).into(),
        );
        let _ = window.configure(rect);
    }

    fn new_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(wl) = window.wl_surface() {
            if !self.unmanaged_surfaces.contains(&wl) {
                self.unmanaged_surfaces.push(wl);
            }
        }
        log::debug!("XWayland: new_override_redirect_window id={}", window.window_id());
    }

    fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let wl = window.wl_surface();
        let window_id = window.window_id();
        let title = window.title();
        let class = window.class();
        let window_type = window.window_type();
        let transient_parent = window.is_transient_for();
        let skip_focus = matches!(window_type, Some(WmWindowType::Tooltip | WmWindowType::Notification | WmWindowType::PopupMenu | WmWindowType::DropdownMenu | WmWindowType::Menu));
        let _ = window.set_mapped(true);
        log::info!("XWayland: map_window_request granted id={} type={:?} transient={:?}", window_id, window_type, transient_parent);

        if let Some(wl) = wl {
            if !self.wl_to_window.contains_key(&wl) {
                let (sw, sh) = self.usable_screen_size();
                let default_w = (sw as f32 * 0.8) as i32;
                let default_h = (sh as f32 * 0.8) as i32;
                let geometry = window.geometry();
                let is_dialog = window_type == Some(WmWindowType::Dialog);
                let (rect, pos) = if geometry.size.w == 0 || geometry.size.h == 0 || is_dialog {
                    let (target_w, target_h) = if is_dialog {
                        ((sw as f32 * 0.5) as i32, (sh as f32 * 0.4) as i32)
                    } else {
                        (default_w, default_h)
                    };
                    let (lx, ly) = if is_dialog {
                        if let Some(parent_id) = transient_parent {
                            if let Some(parent_wl) = self.x11_window_to_surface.get(&parent_id) {
                                if let Some(parent_window) = self.wl_to_window.get(parent_wl) {
                                    if let Some(parent_loc) = self.space.element_location(&WindowElement(parent_window.clone())) {
                                        let pw = parent_window.bbox().size.w;
                                        let ph = parent_window.bbox().size.h;
                                        (parent_loc.x + ((pw as i32 - target_w) / 2).max(0),
                                         parent_loc.y + ((ph as i32 - target_h) / 2).max(0))
                                    } else {
                                        ((sw - target_w) / 2, (sh - target_h) / 2)
                                    }
                                } else {
                                    ((sw - target_w) / 2, (sh - target_h) / 2)
                                }
                            } else {
                                ((sw - target_w) / 2, (sh - target_h) / 2)
                            }
                        } else {
                            ((sw - target_w) / 2, (sh - target_h) / 2)
                        }
                    } else if let Some(parent_id) = transient_parent {
                        if let Some(parent_wl) = self.x11_window_to_surface.get(&parent_id) {
                            if let Some(parent_window) = self.wl_to_window.get(parent_wl) {
                                if let Some(parent_loc) = self.space.element_location(&WindowElement(parent_window.clone())) {
                                    let pw = parent_window.bbox().size.w;
                                    let ph = parent_window.bbox().size.h;
                                    let off_w = (pw as f32 * 0.1) as i32;
                                    let off_h = (ph as f32 * 0.1) as i32;
                                    (parent_loc.x + off_w, parent_loc.y + off_h)
                                } else {
                                    ((sw - default_w) / 2, self.reserved_top + 40)
                                }
                            } else {
                                ((sw - default_w) / 2, self.reserved_top + 40)
                            }
                        } else {
                            ((sw - default_w) / 2, self.reserved_top + 40)
                        }
                    } else {
                        let offset_count = self.wl_to_window.len() as i32;
                        let cascade = 30;
                        let base_x = (sw - default_w) / 2;
                        let base_y = self.reserved_top + 40;
                        (base_x + (offset_count % 10) * cascade, base_y + (offset_count % 10) * cascade)
                    };
                    let rect = smithay::utils::Rectangle::new(
                        (lx.max(0), ly.max(self.reserved_top)).into(),
                        (target_w.max(100), target_h.max(100)).into(),
                    );
                    (rect, (lx.max(0), ly.max(self.reserved_top)))
                } else {
                    (geometry, (geometry.loc.x, geometry.loc.y))
                };
                let _ = window.configure(rect);
                let ping_ts = SERIAL_COUNTER.next_serial().into();
                let _ = window.send_ping(ping_ts);
                self.x11_pending_pings.insert(window_id, ping_ts);
                self.x11_window_to_surface.insert(window_id, wl.clone());
                let x11_window = Window::new_x11_window(window);
                let elem = WindowElement(x11_window.clone());
                self.space.map_element(elem, pos, true);
                self.wl_to_window.insert(wl.clone(), x11_window);

                if !skip_focus && !self.foreign_toplevel_handles.contains_key(&wl) {
                    let app_id = if class.is_empty() { "x11" } else { &class };
                    let ft = self.foreign_toplevel_list_state.new_toplevel::<Self>(title, app_id);
                    ft.send_done();
                    self.foreign_toplevel_handles.insert(wl.clone(), ft);
                }
            }
            if !skip_focus {
                self.focused_surface = Some(wl.clone());
                if let Some(keyboard) = self.keyboard.clone() {
                    keyboard.set_focus(self, Some(wl), SERIAL_COUNTER.next_serial());
                }
            }
        }
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(wl) = window.wl_surface() {
            if !self.unmanaged_surfaces.contains(&wl) {
                self.unmanaged_surfaces.push(wl.clone());
            }
        }
    }

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        let wid = window.window_id();
        if let Some(wl) = window.wl_surface() {
            self.x11_window_to_surface.remove(&wid);
            self.unmanaged_surfaces.retain(|s| s != &wl);
            if let Some(x11_window) = self.wl_to_window.remove(&wl) {
                self.space.unmap_elem(&WindowElement(x11_window));
            }
            self.minimized.remove(&wl);
            self.maximize_restore.remove(&wl);
            self.fullscreen_restore.remove(&wl);
            if self.focused_surface.as_ref() == Some(&wl) {
                self.focused_surface = None;
                let candidate = self.choose_focus_candidate();
                let _ = self.apply_focus_candidate("xwayland_unmapped", candidate);
            }
            if let Some(ft) = self.foreign_toplevel_handles.remove(&wl) {
                self.foreign_toplevel_list_state.remove_toplevel(&ft);
            }
        }
        self.popups.cleanup();
        log::debug!("XWayland: unmapped_window id={}", window.window_id());
    }

    fn destroyed_window(&mut self, _xwm: XwmId, window: X11Surface) {
        let wid = window.window_id();
        if let Some(wl) = window.wl_surface() {
            self.x11_window_to_surface.remove(&wid);
            self.unmanaged_surfaces.retain(|s| s != &wl);
            if let Some(x11_window) = self.wl_to_window.remove(&wl) {
                self.space.unmap_elem(&WindowElement(x11_window));
            }
            self.minimized.remove(&wl);
            self.maximize_restore.remove(&wl);
            self.fullscreen_restore.remove(&wl);
            if self.popup_grab_surface.as_ref() == Some(&wl) {
                self.popup_grab_active = false;
                self.popup_grab_surface = None;
            }
            if self.focused_surface.as_ref() == Some(&wl) {
                self.focused_surface = None;
                let candidate = self.choose_focus_candidate();
                let _ = self.apply_focus_candidate("xwayland_destroyed", candidate);
            }
            if let Some(ft) = self.foreign_toplevel_handles.remove(&wl) {
                self.foreign_toplevel_list_state.remove_toplevel(&ft);
            }
        }
        self.popups.cleanup();
        log::debug!("XWayland: destroyed_window id={}", window.window_id());
    }

    fn configure_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        let (sw, sh) = self.usable_screen_size();
        let default_w = (sw as f32 * 0.8) as i32;
        let default_h = (sh as f32 * 0.8) as i32;
        let (mut fw, mut fh) = (
            w.map(|v| v as i32).unwrap_or(default_w),
            h.map(|v| v as i32).unwrap_or(default_h),
        );

        let min_size = window.min_size();
        let max_size = window.max_size();
        let base_size = window.base_size();

        if let Some(min) = min_size {
            fw = fw.max(min.w);
            fh = fh.max(min.h);
        }
        if let Some(max) = max_size {
            if max.w > 0 {
                fw = fw.min(max.w);
            }
            if max.h > 0 {
                fh = fh.min(max.h);
            }
        }
        if let Some(base) = base_size {
            if fw < base.w {
                fw = base.w;
            }
            if fh < base.h {
                fh = base.h;
            }
        }

        let current_pos = window.wl_surface().and_then(|wl| {
            self.wl_to_window.get(&wl)
                .and_then(|w| self.space.element_location(&WindowElement(w.clone())))
        });
        let (fx, fy) = if let Some(pos) = current_pos {
            (pos.x, pos.y)
        } else {
            (x.unwrap_or((sw - fw) / 2), y.unwrap_or(self.reserved_top + 40))
        };
        let rect = smithay::utils::Rectangle::new(
            (fx.max(0), fy.max(self.reserved_top)).into(),
            (fw.max(100), fh.max(100)).into(),
        );
        let _ = window.configure(rect);
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        geometry: smithay::utils::Rectangle<i32, smithay::utils::Logical>,
        _above: Option<smithay::xwayland::xwm::X11Window>,
    ) {
        log::debug!("XWayland: configure_notify id={} geometry={}x{}+{}+{}",
            window.window_id(), geometry.size.w, geometry.size.h,
            geometry.loc.x, geometry.loc.y);
    }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        _button: u32,
        _resize_edge: ResizeEdge,
    ) {
        log::debug!("XWayland: resize_request id={} (X11Wm disabled, ignored)", window.window_id());
    }

    fn move_request(&mut self, _xwm: XwmId, window: X11Surface, _button: u32) {
        log::debug!("XWayland: move_request id={} (X11Wm disabled, ignored)", window.window_id());
    }

    fn maximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let (w, h) = self.usable_screen_size();
        let rect = smithay::utils::Rectangle::new((0, self.reserved_top).into(), (w, h).into());
        let _ = window.configure(rect);
        let _ = window.set_maximized(true);
    }

    fn unmaximize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_maximized(false);
    }

    fn fullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(wl) = window.wl_surface() {
            if !self.fullscreen_restore.contains_key(&wl) {
                let geom = window.geometry();
                if geom.size.w > 0 && geom.size.h > 0 {
                    self.fullscreen_restore.insert(wl.clone(), geom);
                }
            }
        }
        let mode = self.output.current_mode().unwrap_or(OutputMode {
            size: (1080, 1920).into(),
            refresh: 60000,
        });
        let rect = smithay::utils::Rectangle::new((0, 0).into(), (mode.size.w, mode.size.h).into());
        let _ = window.configure(rect);
        let _ = window.set_fullscreen(true);
    }

    fn unfullscreen_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(wl) = window.wl_surface() {
            if let Some(saved) = self.fullscreen_restore.remove(&wl) {
                let _ = window.configure(saved);
            }
        }
        let _ = window.set_fullscreen(false);
    }

    fn active_window_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        _timestamp: u32,
        currently_active_window: Option<X11Surface>,
    ) {
        if let Some(old_x11) = currently_active_window {
            let _ = old_x11.set_activated(false);
        }
        if let Some(wl) = window.wl_surface() {
            let old_focused = self.focused_surface.clone();
            if old_focused.as_ref() != Some(&wl) {
                if let Some(old_surface) = &old_focused {
                    if let Some(old_w) = self.wl_to_window.get(old_surface) {
                        old_w.set_activated(false);
                    }
                }
            }
            self.focused_surface = Some(wl.clone());
            if let Some(x11_window) = self.wl_to_window.get(&wl) {
                x11_window.set_activated(true);
            }
            if let Some(keyboard) = self.keyboard.clone() {
                keyboard.set_focus(self, Some(wl), SERIAL_COUNTER.next_serial());
            }
            log::debug!("XWayland: active_window_request id={}", window.window_id());
        }
    }

    fn allow_selection_access(
        &mut self,
        _xwm: XwmId,
        _selection: smithay::wayland::selection::SelectionTarget,
    ) -> bool {
        true
    }

    fn send_selection(
        &mut self,
        _xwm: XwmId,
        selection: smithay::wayland::selection::SelectionTarget,
        mime_type: String,
        fd: std::os::unix::io::OwnedFd,
    ) {
        match selection {
            SelectionTarget::Clipboard => {
                let _ = request_data_device_client_selection::<AndroidSeatRuntime>(
                    &self.seat, mime_type, fd,
                );
            }
            SelectionTarget::Primary => {}
        }
    }

    fn new_selection(
        &mut self,
        _xwm: XwmId,
        selection: smithay::wayland::selection::SelectionTarget,
        _mime_types: Vec<String>,
    ) {
        if selection != SelectionTarget::Clipboard {
            return;
        }
        let (read_fd, write_fd) = match nix::unistd::pipe2(nix::fcntl::OFlag::O_CLOEXEC) {
            Ok(fds) => fds,
            Err(e) => {
                log::warn!("X11 clipboard: failed to create pipe: {}", e);
                return;
            }
        };
        if self.x11_wm.as_mut().map_or(true, |wm| {
            wm.send_selection(selection, "text/plain;charset=utf-8".into(), write_fd).is_err()
        }) {
            log::warn!("X11 clipboard: xwm.send_selection failed");
            return;
        }
        let clipboard = self.clipboard_text.clone();
        let dh = self.display_handle.clone();
        let seat = self.seat.clone();
        std::thread::spawn(move || {
            use std::os::fd::{FromRawFd, IntoRawFd};
            let mut file = unsafe { std::fs::File::from_raw_fd(read_fd.into_raw_fd()) };
            let mut buf = String::new();
            if file.read_to_string(&mut buf).is_ok() {
                let text = buf.trim_end_matches('\0').to_string();
                if !text.is_empty() {
                    log::debug!("X11 clipboard: X11 client set selection len={}", text.len());
                    if let Ok(mut guard) = clipboard.lock() {
                        *guard = text.clone();
                    }
                    let mime_types = vec![
                        "text/plain;charset=utf-8".to_string(),
                        "text/plain".to_string(),
                    ];
                    set_data_device_selection::<AndroidSeatRuntime>(
                        &dh, &seat, mime_types, text.clone(),
                    );
                    crate::android::bridge_clipboard::set_clipboard_text(&text);
                    crate::android::command_channel::send_command(
                        crate::android::command_channel::JniCommand::WaylandClipboardToAndroid { text },
                    );
                }
            }
        });
    }

    fn cleared_selection(
        &mut self,
        _xwm: XwmId,
        _selection: smithay::wayland::selection::SelectionTarget,
    ) {
        log::debug!("X11 clipboard: selection cleared");
    }

    fn disconnected(&mut self, _xwm: XwmId) {
        let surfaces: Vec<_> = self.wl_to_window.keys().cloned().collect();
        for wl in &surfaces {
            if let Some(x11_window) = self.wl_to_window.remove(wl) {
                self.space.unmap_elem(&WindowElement(x11_window));
            }
            self.minimized.remove(wl);
            self.maximize_restore.remove(wl);
            self.unmanaged_surfaces.retain(|s| s != wl);
        }
        self.focused_surface = None;
        log::warn!("XWayland: X11 WM disconnected, cleaned up {} surfaces", surfaces.len());
    }

    fn property_notify(&mut self, _xwm: XwmId, window: X11Surface, property: smithay::xwayland::xwm::WmWindowProperty) {
        let wid = window.window_id();
        match property {
            WmWindowProperty::Title => {
                let new_title = window.title();
                log::debug!("XWayland: property_notify id={} title=\"{}\"", wid, new_title);
                if let Some(wl) = window.wl_surface() {
                    if let Some(ft) = self.foreign_toplevel_handles.get(&wl) {
                        ft.send_title(&new_title);
                        ft.send_done();
                    }
                }
            }
            WmWindowProperty::Class => {
                let new_class = window.class();
                log::debug!("XWayland: property_notify id={} class=\"{}\"", wid, new_class);
                if let Some(wl) = window.wl_surface() {
                    if let Some(ft) = self.foreign_toplevel_handles.get(&wl) {
                        let app_id = if new_class.is_empty() { "x11" } else { &new_class };
                        ft.send_app_id(app_id);
                        ft.send_done();
                    }
                }
            }
            _ => {
                log::debug!("XWayland: property_notify id={} property={:?} (ignored)", wid, property);
            }
        }
        let _ = (window, property);
    }

    fn minimize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(wl) = window.wl_surface() {
            self.toggle_minimize(wl.clone());
        }
    }

    fn unminimize_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(wl) = window.wl_surface() {
            if self.minimized.contains_key(&wl) {
                self.toggle_minimize(wl.clone());
            }
        }
    }

    fn above_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(wl) = window.wl_surface() {
            if let Some(x11_window) = self.wl_to_window.get(&wl) {
                let loc = self.space.element_location(&WindowElement(x11_window.clone()));
                self.space.relocate_element(&WindowElement(x11_window.clone()), loc.unwrap_or(smithay::utils::Point::from((0, self.reserved_top))));
            }
        }
        log::debug!("XWayland: above_request id={}", window.window_id());
    }

    fn below_request(&mut self, _xwm: XwmId, window: X11Surface) {
        if let Some(wl) = window.wl_surface() {
            if let Some(x11_window) = self.wl_to_window.get(&wl) {
                if let Some(loc) = self.space.element_location(&WindowElement(x11_window.clone())) {
                    self.space.relocate_element(&WindowElement(x11_window.clone()), (loc.x, self.reserved_top + 1000));
                }
            }
        }
        log::debug!("XWayland: below_request id={}", window.window_id());
    }

    fn unabove_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_above(false);
        log::debug!("XWayland: unabove_request id={}", window.window_id());
    }

    fn unbelow_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_below(false);
        log::debug!("XWayland: unbelow_request id={}", window.window_id());
    }

    fn ping_acked(&mut self, _xwm: XwmId, window: X11Surface, _timestamp: u32) {
        let wid = window.window_id();
        self.x11_pending_pings.remove(&wid);
        log::debug!("XWayland: ping_acked id={}", wid);
    }

    fn sync_request_timeout(&mut self, _xwm: XwmId, window: X11Surface) {
        log::warn!("XWayland: sync timeout id={}", window.window_id());
    }

    fn demands_attention_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_demands_attention(true);
        log::info!("XWayland: demands_attention id={}", window.window_id());
    }

    fn undemands_attention_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_demands_attention(false);
        log::debug!("XWayland: undemands_attention id={}", window.window_id());
    }

    fn stick_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_sticky(true);
        log::debug!("XWayland: stick_request id={}", window.window_id());
    }

    fn unstick_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_sticky(false);
        log::debug!("XWayland: unstick_request id={}", window.window_id());
    }

    fn shade_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_shaded(true);
        log::debug!("XWayland: shade_request id={}", window.window_id());
    }

    fn unshade_request(&mut self, _xwm: XwmId, window: X11Surface) {
        let _ = window.set_shaded(false);
        log::debug!("XWayland: unshade_request id={}", window.window_id());
    }

    fn show_desktop_request(&mut self, _xwm: XwmId) {
        log::info!("XWayland: show_desktop_request");
        let surfaces: Vec<_> = self.wl_to_window.keys().cloned().collect();
        for wl in &surfaces {
            if let Some(window) = self.wl_to_window.get(wl) {
                if let Some(x11) = window.x11_surface() {
                    if !x11.is_hidden() && !self.minimized.contains_key(wl) {
                        if let Some(loc) = self.space.element_location(&WindowElement(window.clone())) {
                            self.minimized.insert(wl.clone(), loc);
                        }
                        let _ = x11.set_hidden(true);
                        self.space.unmap_elem(&WindowElement(window.clone()));
                    }
                }
            }
        }
        self.render_all();
    }

    fn unshow_desktop_request(&mut self, _xwm: XwmId) {
        log::info!("XWayland: unshow_desktop_request");
        let minimized: Vec<_> = self.minimized.keys().cloned().collect();
        for wl in &minimized {
            if let Some(pos) = self.minimized.remove(wl) {
                if let Some(window) = self.wl_to_window.get(wl) {
                    if let Some(x11) = window.x11_surface() {
                        let _ = x11.set_hidden(false);
                    }
                    self.space.map_element(WindowElement(window.clone()), pos, false);
                }
            }
        }
        self.render_all();
    }

    fn randr_screen_change_size(&mut self, _xwm: XwmId, new_size: (u16, u16)) {
        let (w, h) = (new_size.0 as i32, new_size.1 as i32);
        log::info!("XRandR: screen size changed to {}x{}", w, h);
        self.update_output_mode(w, h, None);
    }
}
