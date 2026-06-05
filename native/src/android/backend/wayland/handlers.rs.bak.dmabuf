#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::seat::AndroidSeatRuntime;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::server::WaylandClientState;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::shell::WindowElement;
#[cfg(feature = "smithay_android")]
use smithay::backend::renderer::utils::on_commit_buffer_handler;
#[cfg(feature = "smithay_android")]
use smithay::desktop::space::SpaceElement;
#[cfg(feature = "smithay_android")]
use smithay::desktop::utils::send_frames_surface_tree;
#[cfg(feature = "smithay_android")]
use smithay::desktop::{PopupKind, Window};
#[cfg(feature = "smithay_android")]
use smithay::input::dnd::DndGrabHandler;
#[cfg(feature = "smithay_android")]
use smithay::input::pointer::CursorImageStatus;
#[cfg(feature = "smithay_android")]
use smithay::input::pointer::PointerHandle;
#[cfg(feature = "smithay_android")]
use smithay::input::{Seat, SeatHandler, SeatState};
#[cfg(feature = "smithay_android")]
use smithay::output::{Mode as OutputMode, Output};
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_protocols::xdg::shell::server::xdg_toplevel;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::protocol::wl_buffer;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::protocol::wl_output::WlOutput;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::protocol::wl_seat;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::{Client, Resource};
#[cfg(feature = "smithay_android")]
use smithay::utils::{Logical, Point, Serial, Transform, SERIAL_COUNTER};
#[cfg(feature = "smithay_android")]
use smithay::wayland::buffer::BufferHandler;
#[cfg(feature = "smithay_android")]
use smithay::wayland::compositor::{CompositorClientState, CompositorHandler, CompositorState};
#[cfg(feature = "smithay_android")]
use smithay::xwayland::XWaylandClientData;
#[cfg(feature = "smithay_android")]
use smithay::wayland::foreign_toplevel_list::{
    ForeignToplevelListHandler, ForeignToplevelListState,
};
#[cfg(feature = "smithay_android")]
use smithay::wayland::fractional_scale::FractionalScaleHandler;
#[cfg(feature = "smithay_android")]
use smithay::wayland::idle_inhibit::IdleInhibitHandler;
#[cfg(feature = "smithay_android")]
use smithay::wayland::input_method::{InputMethodHandler, PopupSurface as InputMethodPopup};
#[cfg(feature = "smithay_android")]
use smithay::wayland::output::OutputHandler;
#[cfg(feature = "smithay_android")]
use smithay::wayland::pointer_constraints::PointerConstraintsHandler;
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::data_device::{
    set_data_device_focus, DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler,
};
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::primary_selection::{
    PrimarySelectionHandler, PrimarySelectionState,
};
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::wlr_data_control::{DataControlHandler, DataControlState};
#[cfg(feature = "smithay_android")]
use smithay::wayland::shell::wlr_layer::{LayerSurface, WlrLayerShellHandler, WlrLayerShellState};
#[cfg(feature = "smithay_android")]
use smithay::wayland::shell::xdg::decoration::XdgDecorationHandler;
use smithay::wayland::shell::xdg::XdgToplevelSurfaceData;
#[cfg(feature = "smithay_android")]
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
};
#[cfg(feature = "smithay_android")]
use smithay::wayland::shm::{ShmHandler, ShmState};
#[cfg(feature = "smithay_android")]
use smithay::wayland::tablet_manager::TabletSeatHandler;
#[cfg(feature = "smithay_android")]
use smithay::wayland::xdg_activation::{
    XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
};
#[cfg(feature = "smithay_android")]
// تم إزالة FALLBACK_CLIENT_COMPOSITOR_STATE لأنه كان يسبب مشكلة

// ── Smithay handler trait implementations ─────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl SeatHandler for AndroidSeatRuntime {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        match &image {
            CursorImageStatus::Hidden => self.last_cursor_mode = "hidden".to_string(),
            CursorImageStatus::Named(icon) => self.last_cursor_mode = format!("named:{:?}", icon),
            CursorImageStatus::Surface(_) => self.last_cursor_mode = "surface".to_string(),
        }
        self.cursor_status = match image {
            CursorImageStatus::Hidden => None,
            other => Some(other),
        };
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let client = focused.and_then(|s| self.display_handle.get_client(s.id()).ok());
        set_data_device_focus(&self.display_handle, seat, client);

        if focused.is_none() {
            self.cursor_status = None;
            self.last_cursor_mode = "fallback:named:Default".to_string();
        }
    }
}

#[cfg(feature = "smithay_android")]
impl OutputHandler for AndroidSeatRuntime {
    fn output_bound(&mut self, output: Output, _wl_output: WlOutput) {
        // نستخدم الشاشة المرتبطة نفسها، وليس self.output
        let mode = output.current_mode().unwrap_or(OutputMode {
            size: (1080, 1920).into(),
            refresh: 60000,
        });
        output.change_current_state(Some(mode), Some(Transform::Normal), None, None);
        log::info!(
            "SmithayRuntime: output_bound updated output {:?} to mode {:?}",
            output.name(),
            mode
        );
    }
}

#[cfg(feature = "smithay_android")]
impl IdleInhibitHandler for AndroidSeatRuntime {
    fn inhibit(&mut self, surface: WlSurface) {
        self.idle_inhibit_count = self.idle_inhibit_count.saturating_add(1);
        log::debug!(
            "IdleInhibit: inhibit added (total={}) surface={:?}",
            self.idle_inhibit_count,
            surface.id()
        );
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        self.idle_inhibit_count = self.idle_inhibit_count.saturating_sub(1);
        log::debug!(
            "IdleInhibit: inhibit removed (total={}) surface={:?}",
            self.idle_inhibit_count,
            surface.id()
        );
    }
}

#[cfg(feature = "smithay_android")]
impl InputMethodHandler for AndroidSeatRuntime {
    fn new_popup(&mut self, surface: InputMethodPopup) {
        let wl = surface.wl_surface().clone();
        if !self.unmanaged_surfaces.contains(&wl) {
            self.unmanaged_surfaces.push(wl);
        }
        log::debug!("InputMethod: new popup surface registered");
    }

    fn dismiss_popup(&mut self, surface: InputMethodPopup) {
        let wl = surface.wl_surface();
        self.unmanaged_surfaces.retain(|s| s != wl);
        if self.focused_surface.as_ref() == Some(wl) {
            self.focused_surface = None;
        }
        log::debug!("InputMethod: popup dismissed");
    }

    fn popup_repositioned(&mut self, _surface: InputMethodPopup) {
        log::debug!("InputMethod: popup repositioned");
    }

    fn parent_geometry(
        &self,
        parent: &WlSurface,
    ) -> smithay::utils::Rectangle<i32, smithay::utils::Logical> {
        if let Some(window) = self.wl_to_window.get(parent) {
            let elem = WindowElement(window.clone());
            let loc = self.space.element_location(&elem);
            let bbox = elem.bbox();
            if let Some(loc) = loc {
                return smithay::utils::Rectangle::new(
                    (loc.x, loc.y).into(),
                    (bbox.size.w, bbox.size.h).into(),
                );
            }
        }
        let mode = self.output.current_mode().unwrap_or(OutputMode {
            size: (1080, 1920).into(),
            refresh: 60000,
        });
        smithay::utils::Rectangle::new((0, 0).into(), (mode.size.w, mode.size.h).into())
    }
}

#[cfg(feature = "smithay_android")]
impl DataDeviceHandler for AndroidSeatRuntime {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.data_device_state
    }
}

#[cfg(feature = "smithay_android")]
impl DndGrabHandler for AndroidSeatRuntime {}

#[cfg(feature = "smithay_android")]
impl WaylandDndGrabHandler for AndroidSeatRuntime {}

#[cfg(feature = "smithay_android")]
impl CompositorHandler for AndroidSeatRuntime {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        if let Some(state) = client.get_data::<WaylandClientState>() {
            &state.compositor_state
        } else if let Some(state) = client.get_data::<XWaylandClientData>() {
            &state.compositor_state
        } else {
            // This path should never be reached: Wayland clients get WaylandClientState
            // from insert_client, XWayland clients get XWaylandClientData from Smithay.
            // The FALLBACK static is safe because it holds default (scale=1.0).
            log::warn!("SmithayRuntime: Client has neither WaylandClientState nor XWaylandClientData, using default fallback");
            static FALLBACK: std::sync::OnceLock<CompositorClientState> =
                std::sync::OnceLock::new();
            FALLBACK.get_or_init(CompositorClientState::default)
        }
    }

    fn commit(&mut self, surface: &WlSurface) {
        self.prune_dead_surfaces();

        self.popups.commit(surface);

        let is_x11 = self.wl_to_window.get(surface).and_then(|w| w.x11_surface()).is_some();
        let is_new_unmanaged = if let Some(window) = self.wl_to_window.get(surface) {
            window.on_commit();
            false
        } else if !self.unmanaged_surfaces.contains(surface) {
            self.unmanaged_surfaces.push(surface.clone());
            true
        } else {
            false
        };

        if is_x11 {
            use smithay::wayland::compositor::with_states;
            with_states(surface, |states| {
                let mut attrs = states.cached_state.get::<smithay::wayland::compositor::SurfaceAttributes>();
                log::info!("XWayland commit: surface={:?} has_buffer={}",
                    surface.id(),
                    attrs.current().buffer.is_some(),
                );
            });
        }

        if is_new_unmanaged {
            for layer in self.layer_shell_state.layer_surfaces() {
                if layer.wl_surface() == surface {
                    let (screen_w, screen_h) = crate::android::command_channel::get_surface_size();
                    let (screen_w, screen_h) = if screen_w > 0 && screen_h > 0 {
                        (screen_w, screen_h)
                    } else {
                        (1080, 1920)
                    };
                    let mut configure_w = screen_w;
                    let mut configure_h = 0;

                    layer.with_cached_state(|cached| {
                        let anchor = cached.anchor;
                        let margin = cached.margin;
                        let desired = cached.size;

                        let anchored_h = anchor.anchored_horizontally();
                        let anchored_v = anchor.anchored_vertically();

                        if anchored_h {
                            configure_w = (screen_w - margin.left - margin.right).max(1);
                        } else if desired.w > 0 {
                            configure_w = desired.w;
                        }

                        if anchored_v {
                            configure_h = (screen_h
                                - margin.top
                                - margin.bottom
                                - self.reserved_top
                                - self.reserved_bottom)
                                .max(1);
                            if desired.h > 0 && desired.h < configure_h {
                                configure_h = desired.h;
                            }
                        } else if desired.h > 0 {
                            configure_h = desired.h;
                        }

                        let ez: i32 = cached.exclusive_zone.into();
                        match cached.layer {
                            smithay::wayland::shell::wlr_layer::Layer::Top if ez > 0 => {
                                self.reserved_top = ez; // سيتم إعادة الحساب الكامل بعد commit
                            }
                            smithay::wayland::shell::wlr_layer::Layer::Bottom if ez > 0 => {
                                self.reserved_bottom = ez;
                            }
                            _ => {}
                        }
                    });

                    // إعادة حساب المساحة المحجوزة من جميع الطبقات
                    self.recalculate_reserved_zones();

                    layer.with_pending_state(|state| {
                        state.size = Some((configure_w, configure_h).into());
                    });
                    layer.send_configure();
                    log::info!(
                        "SmithayRuntime: layer-surface committed configured {}x{}",
                        configure_w,
                        configure_h
                    );
                    break;
                }
            }
        }

        // التركيز التلقائي فقط إذا كان السطح من نوع toplevel وليس layer أو unmanaged من نوع آخر
        let is_toplevel = self.wl_to_window.contains_key(surface);
        let is_layer = self
            .layer_shell_state
            .layer_surfaces()
            .any(|l| l.wl_surface() == surface);
        if self.focused_surface.is_none() && surface.is_alive() && is_toplevel && !is_layer {
            self.focused_surface = Some(surface.clone());
            if let Some(keyboard) = self.keyboard.clone() {
                keyboard.set_focus(self, Some(surface.clone()), SERIAL_COUNTER.next_serial());
            }
            log::info!(
                "SmithayRuntime: auto-focused first committed toplevel surface id={:?}",
                surface.id()
            );
        }

        on_commit_buffer_handler::<Self>(surface);

        send_frames_surface_tree(
            surface,
            &self.output,
            std::time::Duration::ZERO,
            None,
            |_, _| Some(self.output.clone()),
        );

        self.render_all();

        self.sync_text_input_to_android();
    }
}

#[cfg(feature = "smithay_android")]
impl XdgShellHandler for AndroidSeatRuntime {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let wl_surface = surface.wl_surface().clone();

        let (title, app_id) = smithay::wayland::compositor::with_states(&wl_surface, |states| {
            let data = states.data_map.get::<XdgToplevelSurfaceData>();
            let guard = data.and_then(|d| d.lock().ok());
            let title = guard
                .as_ref()
                .and_then(|g| g.title.clone())
                .unwrap_or_else(|| "Window".to_string());
            let app_id = guard
                .as_ref()
                .and_then(|g| g.app_id.clone())
                .unwrap_or_else(|| "unknown".to_string());
            (title, app_id)
        });

        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
            state.states.set(xdg_toplevel::State::Maximized);
            let (w, h) = self.usable_screen_size();
            state.size = Some((w, h).into());
        });
        surface.send_configure();

        let window = Window::new_wayland_window(surface);
        let elem = WindowElement(window.clone());

        let pos = Point::<i32, Logical>::from((0, self.reserved_top));
        self.space.map_element(elem, pos, true);
        self.wl_to_window.insert(wl_surface.clone(), window);

        let ft_handle = self
            .foreign_toplevel_list_state
            .new_toplevel::<Self>(title, app_id);
        ft_handle.send_done();
        self.foreign_toplevel_handles
            .insert(wl_surface.clone(), ft_handle);

        log::info!("SmithayRuntime: xdg toplevel configured, setting focus to surface");
        self.focused_surface = Some(wl_surface.clone());

        if let Some(keyboard) = self.keyboard.clone() {
            keyboard.set_focus(self, Some(wl_surface), SERIAL_COUNTER.next_serial());
        }
    }

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        let _wl_surface = surface.wl_surface().clone();
        let popup_geo = positioner.get_geometry();

        if let Err(e) = self.popups.track_popup(PopupKind::from(surface.clone())) {
            log::warn!("SmithayRuntime: failed to track popup: {:?}", e);
        }

        let _ = surface.send_configure();

        // لا نمنح التركيز التلقائي للpopups
        log::debug!(
            "SmithayRuntime: new xdg_popup size=({}x{})",
            popup_geo.size.w,
            popup_geo.size.h
        );
    }

    fn grab(&mut self, surface: PopupSurface, _seat: wl_seat::WlSeat, _serial: Serial) {
        let wl_surface = surface.wl_surface().clone();
        self.popup_grab_active = true;
        self.popup_grab_surface = Some(wl_surface);
        log::debug!("SmithayRuntime: xdg_popup grab activated");
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        let (w, h) = self.usable_screen_size();
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
            state.states.set(xdg_toplevel::State::Maximized);
            state.size = Some((w, h).into());
            state.bounds = Some((w, h).into());
        });
        surface.send_configure();
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Maximized);
            state.size = None;
        });
        surface.send_configure();
    }

    fn fullscreen_request(&mut self, surface: ToplevelSurface, _output: Option<WlOutput>) {
        // استخدام المساحة القابلة للاستخدام بدلاً من حجم الشاشة الخام
        let (w, h) = self.usable_screen_size();
        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
            state.states.set(xdg_toplevel::State::Fullscreen);
            state.size = Some((w, h).into());
            state.bounds = Some((w, h).into());
        });
        surface.send_configure();
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Fullscreen);
            state.states.set(xdg_toplevel::State::Activated);
        });
        surface.send_configure();
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        let wl = surface.wl_surface().clone();

        if let Some(window) = self.wl_to_window.remove(&wl) {
            self.space.unmap_elem(&WindowElement(window));
        }

        self.popups.cleanup();

        if let Some(handle) = self.foreign_toplevel_handles.remove(&wl) {
            self.foreign_toplevel_list_state.remove_toplevel(&handle);
        }

        self.minimized.remove(&wl);
        self.maximize_restore.remove(&wl);

        if self.popup_grab_surface.as_ref() == Some(&wl) {
            self.popup_grab_active = false;
            self.popup_grab_surface = None;
        }

        if self.gesture_surface.as_ref() == Some(&wl) {
            self.gesture_target = None;
            self.gesture_surface = None;
        }

        if self.focused_surface.as_ref() == Some(&wl) {
            self.focused_surface = None;
            let candidate = self.choose_focus_candidate();
            let _ = self.apply_focus_candidate("toplevel_destroyed", candidate);
        }

        self.prune_dead_surfaces();
    }

    fn title_changed(&mut self, surface: ToplevelSurface) {
        let wl = surface.wl_surface().clone();
        if let Some(handle) = self.foreign_toplevel_handles.get(&wl) {
            let title = smithay::wayland::compositor::with_states(&wl, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .and_then(|d| d.lock().ok())
                    .and_then(|g| g.title.clone())
                    .unwrap_or_else(|| "Window".to_string())
            });
            handle.send_title(&title);
            handle.send_done();
        }
    }

    fn app_id_changed(&mut self, surface: ToplevelSurface) {
        let wl = surface.wl_surface().clone();
        if let Some(handle) = self.foreign_toplevel_handles.get(&wl) {
            let app_id = smithay::wayland::compositor::with_states(&wl, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .and_then(|d| d.lock().ok())
                    .and_then(|g| g.app_id.clone())
                    .unwrap_or_else(|| "unknown".to_string())
            });
            handle.send_app_id(&app_id);
            handle.send_done();
        }
    }
}

#[cfg(feature = "smithay_android")]
impl XdgDecorationHandler for AndroidSeatRuntime {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Activated);
            state.decoration_mode = Some(smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode::ClientSide);
        });
        toplevel.send_configure();
    }
    fn request_mode(
        &mut self,
        toplevel: ToplevelSurface,
        mode: smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
    ) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(mode);
        });
        toplevel.send_configure();
    }
    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(smithay::reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode::ClientSide);
        });
        toplevel.send_configure();
    }
}

#[cfg(feature = "smithay_android")]
impl WlrLayerShellHandler for AndroidSeatRuntime {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        _surface: LayerSurface,
        _output: Option<WlOutput>,
        layer: smithay::wayland::shell::wlr_layer::Layer,
        _namespace: String,
    ) {
        log::debug!(
            "SmithayRuntime: layer_surface created layer={:?} — configure deferred to first commit",
            layer
        );
        // لا نحتاج إلى إعادة الحساب هنا لأن الطبقة لم تلتزم بعد، لكننا سنفعل عند أول commit
    }
}

#[cfg(feature = "smithay_android")]
impl BufferHandler for AndroidSeatRuntime {
    fn buffer_destroyed(&mut self, buffer: &wl_buffer::WlBuffer) {
        log::debug!(
            "Buffer destroyed: {:?} — SHM cleanup managed by on_commit_buffer_handler",
            buffer.id()
        );
    }
}

#[cfg(feature = "smithay_android")]
impl ShmHandler for AndroidSeatRuntime {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

#[cfg(feature = "smithay_android")]
impl FractionalScaleHandler for AndroidSeatRuntime {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        use smithay::wayland::compositor::with_states;
        use smithay::wayland::fractional_scale::with_fractional_scale;
        // نستخدم الشاشة المرتبطة بالسطح، وليس self.output
        // في هذا السياق البسيط، نفترض أن الشاشة الوحيدة هي self.output
        // لكن الأفضل هو استخراج الشاشة من space
        let output = self.space.outputs().next().unwrap_or(&self.output);
        let scale = output.current_scale().fractional_scale();
        with_states(&surface, |states| {
            with_fractional_scale(states, |fs| {
                fs.set_preferred_scale(scale);
            });
        });
        log::debug!(
            "SmithayRuntime: fractional_scale registered for surface id={:?}, preferred={}",
            surface.id(),
            scale,
        );
    }
}

#[cfg(feature = "smithay_android")]
impl TabletSeatHandler for AndroidSeatRuntime {}

#[cfg(feature = "smithay_android")]
impl XdgActivationHandler for AndroidSeatRuntime {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.xdg_activation_state
    }

    fn request_activation(
        &mut self,
        _token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        let token_serial = token_data.serial.map(|(s, _)| s);
        if let Some(ts) = token_serial {
            if let Some(last) = self.last_activation_serial {
                let ts_val: u32 = ts.into();
                let last_val: u32 = last.into();
                let diff = ts_val.wrapping_sub(last_val);
                if diff > (u32::MAX / 2) {
                    log::warn!(
                        "XdgActivation: stale serial (wrapped) ts={:?} last={:?}, ignoring activation for surface {:?}",
                        ts, last, surface.id()
                    );
                    return;
                }
                if ts < last {
                    log::warn!(
                        "XdgActivation: stale serial ({:?}) < last ({:?}), ignoring activation for surface {:?}",
                        ts, last, surface.id()
                    );
                    return;
                }
            }
            self.last_activation_serial = Some(ts);
        }

        log::debug!(
            "SmithayRuntime: xdg_activation request for surface id={:?} serial={:?}",
            surface.id(),
            token_serial
        );
        if surface.is_alive() {
            self.focused_surface = Some(surface.clone());
            if let Some(keyboard) = self.keyboard.clone() {
                keyboard.set_focus(self, Some(surface), SERIAL_COUNTER.next_serial());
            }
        }
    }
}

#[cfg(feature = "smithay_android")]
impl PrimarySelectionHandler for AndroidSeatRuntime {
    fn primary_selection_state(&mut self) -> &mut PrimarySelectionState {
        &mut self.primary_selection_state
    }
}

// ── Pointer constraints ───────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl PointerConstraintsHandler for AndroidSeatRuntime {
    fn new_constraint(&mut self, surface: &WlSurface, _pointer: &PointerHandle<Self>) {
        log::info!(
            "PointerConstraints: new constraint surface={:?} — enforcement delegated to smithay internals + relative-motion dispatch in input_router",
            surface.id()
        );
    }
    fn cursor_position_hint(
        &mut self,
        surface: &WlSurface,
        _pointer: &PointerHandle<Self>,
        location: Point<f64, Logical>,
    ) {
        log::debug!("PointerConstraints: cursor hint surface={:?} @ ({:.1}, {:.1}) — no physical pointer to warp", surface.id(), location.x, location.y);
        // في نظام حقيقي قد نحتاج إلى تحريك المؤشر، لكن هنا لا يوجد فأر فعلي.
    }
}

// ── Foreign toplevel list ──────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl ForeignToplevelListHandler for AndroidSeatRuntime {
    fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelListState {
        &mut self.foreign_toplevel_list_state
    }
}

// ── Data control ────────────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl DataControlHandler for AndroidSeatRuntime {
    fn data_control_state(&mut self) -> &mut DataControlState {
        &mut self.data_control_state
    }
}
