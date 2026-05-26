use crate::android::backend::wayland::engine_timing;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::input::{RoutedInputEvent, TouchPoint};
#[cfg(feature = "smithay_android")]
use smithay::backend::input::{Axis, AxisSource, ButtonState, KeyState};
#[cfg(feature = "smithay_android")]
use smithay::input::keyboard::FilterResult;
#[cfg(feature = "smithay_android")]
use smithay::input::pointer::{
    AxisFrame, ButtonEvent, MotionEvent as PointerMotionEvent, RelativeMotionEvent,
};
#[cfg(feature = "smithay_android")]
use smithay::input::touch::{DownEvent, MotionEvent as TouchMotionEvent, UpEvent};
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
#[cfg(feature = "smithay_android")]
use smithay::desktop::space::SpaceElement;
use smithay::wayland::seat::WaylandFocus;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::Resource;
#[cfg(feature = "smithay_android")]
use smithay::utils::{Logical, Point, SERIAL_COUNTER};
#[cfg(feature = "smithay_android")]
use smithay::xwayland::xwm::ResizeEdge;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::seat::{AndroidSeatRuntime, GestureTarget, WinlandInputMode};
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::shell::WindowElement;
#[cfg(feature = "smithay_android")]
use smithay::wayland::compositor;
#[cfg(feature = "smithay_android")]
use smithay::wayland::xwayland_shell::XWAYLAND_SHELL_ROLE;

#[cfg(feature = "smithay_android")]
fn is_xwayland_surface(surface: &WlSurface) -> bool {
    compositor::get_role(surface) == Some(XWAYLAND_SHELL_ROLE)
}

// ── Public re-exports (used externally from smithay_runtime) ─────────────────

#[cfg(feature = "smithay_android")]
pub use xkbcommon::xkb::{
    Context as XkbContext, Keymap as XkbKeymap, KeymapCompileFlags as XkbKeymapCompileFlags,
};

// ── Input routing and focus management ───────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl AndroidSeatRuntime {
    /// Focus next/previous alive window.
    pub(crate) fn cycle_window_focus(&mut self, direction: i32) {
        let alive: Vec<WlSurface> = self
            .space
            .elements()
            .filter_map(|elem| elem.0.wl_surface().map(|s| s.as_ref().clone()))
            .collect();

        if alive.len() < 2 {
            return;
        }

        let current_idx = self
            .focused_surface
            .as_ref()
            .and_then(|f| alive.iter().position(|s| s == f))
            .unwrap_or(0);

        let next_idx = if direction >= 0 {
            (current_idx + 1) % alive.len()
        } else {
            (current_idx + alive.len() - 1) % alive.len()
        };

        let _ = self.apply_focus_candidate(
            "gesture_swipe_cycle",
            Some(alive[next_idx].clone()),
        );
    }

    pub(crate) fn choose_focus_candidate(&self) -> Option<WlSurface> {
        self.space
            .elements()
            .rev()
            .filter_map(|elem| elem.0.wl_surface().map(|s| s.as_ref().clone()))
            .find(|s| s.is_alive())
            .or_else(|| self.focused_surface.as_ref().filter(|s| s.is_alive()).cloned())
    }

    pub(crate) fn choose_focus_at_point(&self, x: f32, y: f32) -> Option<WlSurface> {
        let point: Point<f64, Logical> = (x as f64, y as f64).into();
        let hit = self.space.element_under(point).and_then(|(elem, _)| {
            elem.0.wl_surface().map(|s| s.as_ref().clone())
        });
        hit.or_else(|| self.choose_focus_candidate())
    }

    pub(crate) fn apply_forced_focus_at(&mut self, reason: &str, x: f32, y: f32) -> Option<WlSurface> {
        let candidate = self.choose_focus_at_point(x, y);
        self.apply_focus_candidate(reason, candidate)
    }

    pub(crate) fn apply_forced_focus(&mut self, reason: &str) -> Option<WlSurface> {
        let candidate = self.choose_focus_candidate();
        self.apply_focus_candidate(reason, candidate)
    }

    pub(crate) fn apply_focus_candidate(&mut self, reason: &str, candidate: Option<WlSurface>) -> Option<WlSurface> {

        let Some(target) = candidate.clone() else {
            self.last_focus_decision = format!(
                "reason={} action=no-surface windows={} had_focus={}",
                reason,
                self.space.elements().count(),
                self.focused_surface.is_some()
            );
            return None;
        };

        let switched = self.focused_surface.as_ref() != Some(&target);
        self.focused_surface = Some(target.clone());

        if let Some(keyboard) = self.keyboard.clone() {
            keyboard.set_focus(self, Some(target.clone()), SERIAL_COUNTER.next_serial());
        }

        self.last_focus_decision = format!(
            "reason={} action=focused switched={} windows={}",
            reason,
            switched,
            self.space.elements().count()
        );

        Some(target)
    }

    pub(crate) fn ensure_focus_for_non_pointer(&mut self, reason: &str) {
        if self.focused_surface.is_none() {
            let _ = self.apply_forced_focus(reason);
        }
    }

    pub(crate) fn update_modifier_state_from_android_key(&mut self, keycode: i32, is_down: bool) -> bool {
        let before = self.android_modifiers;
        match keycode {
            57 | 58 => self.android_modifiers.alt = is_down,
            59 | 60 => self.android_modifiers.shift = is_down,
            113 | 114 => self.android_modifiers.ctrl = is_down,
            117 | 118 => self.android_modifiers.logo = is_down,
            115 if is_down => self.android_modifiers.caps_lock = !self.android_modifiers.caps_lock,
            143 if is_down => self.android_modifiers.num_lock = !self.android_modifiers.num_lock,
            _ => {}
        }

        self.android_modifiers != before
    }

    pub(crate) fn sync_keyboard_modifiers(&mut self, reason: &str) {
        let Some(keyboard) = self.keyboard.clone() else {
            return;
        };

        let changed_mask = keyboard.set_modifier_state(self.android_modifiers);
        if changed_mask != 0 {
            keyboard.advertise_modifier_state(self);
            self.last_seat_dispatch = format!(
                "mods_sync reason={} mask={} shift={} ctrl={} alt={} logo={} caps={} num={}",
                reason,
                changed_mask,
                self.android_modifiers.shift,
                self.android_modifiers.ctrl,
                self.android_modifiers.alt,
                self.android_modifiers.logo,
                self.android_modifiers.caps_lock,
                self.android_modifiers.num_lock
            );
        }
    }

    fn dispatch_touch_down(&mut self, id: i32, point: &TouchPoint) {
        // Popup dismissal
        if self.popup_grab_active {
            let outside = if let Some(ref _grab_surface) = self.popup_grab_surface {
                let pt: Point<f64, Logical> = (point.x as f64, point.y as f64).into();
                self.space.element_under(pt).is_none()
            } else {
                true
            };
            if outside {
                if let Some(ref grab_surface) = self.popup_grab_surface.clone() {
                    self.dismiss_popup(grab_surface);
                }
                self.popup_grab_active = false;
                self.popup_grab_surface = None;
                self.last_seat_dispatch = format!("popup_dismiss id={}", id);
            }
        }

        // Gesture detection: titlebar buttons, window edges, drag
        if self.gesture_target.is_none() {
            if let Some(ref focus) = self.focused_surface.clone() {
                if let Some(window) = self.wl_to_window.get(focus) {
                    let elem = WindowElement(window.clone());
                    if let Some(loc) = self.space.element_location(&elem) {
                        let bbox = elem.bbox();
                        let fx = loc.x as f32;
                        let fy = loc.y as f32;
                        let fw = bbox.size.w as f32;
                        let fh = bbox.size.h as f32;
                        if fh > 0.0 && point.x >= fx && point.x <= fx + fw {
                            let titlebar_h = 24.0;
                            if point.y >= fy - titlebar_h && point.y <= fy {
                                if point.x >= fx + 8.0 && point.x <= fx + 18.0 {
                                    self.close_surface(focus.clone());
                                    self.last_seat_dispatch = format!("close_btn id={}", id);
                                    self.active_touch_ids.insert(id);
                                    return;
                                }
                                if point.x >= fx + 24.0 && point.x <= fx + 34.0 {
                                    self.toggle_minimize(focus.clone());
                                    self.last_seat_dispatch = format!("minimize_btn id={}", id);
                                    self.active_touch_ids.insert(id);
                                    return;
                                }
                                if point.x >= fx + 40.0 && point.x <= fx + 50.0 {
                                    self.toggle_maximize(focus.clone());
                                    self.last_seat_dispatch = format!("maximize_btn id={}", id);
                                    self.active_touch_ids.insert(id);
                                    return;
                                }
                                self.gesture_target = Some(GestureTarget::Move);
                                self.gesture_surface = Some(focus.clone());
                                self.gesture_origin = (point.x, point.y);
                                self.active_touch_ids.insert(id);
                                self.last_seat_dispatch = format!("titlebar_drag id={} x={:.0} y={:.0}", id, point.x, point.y);
                                return;
                            }
                            let edge_px = 20.0;
                            let edge = if point.y >= fy + fh - edge_px && point.y <= fy + fh {
                                let is_left = point.x <= fx + edge_px;
                                let is_right = point.x >= fx + fw - edge_px;
                                match (is_left, is_right) {
                                    (true, _) => Some(ResizeEdge::BottomLeft),
                                    (_, true) => Some(ResizeEdge::BottomRight),
                                    _ => Some(ResizeEdge::Bottom),
                                }
                            } else if point.x >= fx && point.x <= fx + edge_px {
                                Some(ResizeEdge::Left)
                            } else if point.x >= fx + fw - edge_px && point.x <= fx + fw {
                                Some(ResizeEdge::Right)
                            } else {
                                None
                            };
                            if let Some(edge) = edge {
                                self.gesture_target = Some(GestureTarget::Resize(edge));
                                self.gesture_surface = Some(focus.clone());
                                self.gesture_origin = (point.x, point.y);
                                self.active_touch_ids.insert(id);
                                self.last_seat_dispatch = format!("resize_start id={} edge={:?}", id, edge);
                                return;
                            }
                        }
                    }
                }
            }
        }
        if self.gesture_target.is_some() {
            self.gesture_origin = (point.x, point.y);
            self.active_touch_ids.insert(id);
            self.last_seat_dispatch = format!("gesture_touch_down captured id={}", id);
            return;
        }
    }

    fn dispatch_touch_move_gesture(&mut self, id: i32, point: &TouchPoint) -> bool {
        if let Some(target) = self.gesture_target {
            if let Some(ref surface) = self.gesture_surface.clone() {
                let dx = (point.x - self.gesture_origin.0) as i32;
                let dy = (point.y - self.gesture_origin.1) as i32;
                if let Some(window) = self.wl_to_window.get(surface).cloned() {
                    let elem = WindowElement(window.clone());
                    match target {
                        GestureTarget::Move => {
                            if let Some(loc) = self.space.element_location(&elem) {
                                self.space.relocate_element(&elem, (loc.x + dx, loc.y + dy));
                            }
                        }
                        GestureTarget::Resize(edge) => {
                            let (_sw, _sh) = self.usable_screen_size();
                            let bbox = elem.bbox();
                            let mut nw = bbox.size.w + dx;
                            let mut nh = bbox.size.h + dy;
                            let mut nx = bbox.loc.x;
                            let mut ny = bbox.loc.y;
                            match edge {
                                ResizeEdge::Right => nw = nw.max(100),
                                ResizeEdge::Left => {
                                    let new_w = nw.max(100);
                                    nx += bbox.size.w - new_w;
                                    nw = new_w;
                                }
                                ResizeEdge::Bottom => nh = nh.max(100),
                                ResizeEdge::Top => {
                                    let new_h = nh.max(100);
                                    ny += bbox.size.h - new_h;
                                    nh = new_h;
                                }
                                ResizeEdge::BottomRight => { nw = nw.max(100); nh = nh.max(100); }
                                ResizeEdge::BottomLeft => {
                                    let new_w = nw.max(100);
                                    nx += bbox.size.w - new_w; nw = new_w; nh = nh.max(100);
                                }
                                ResizeEdge::TopRight => {
                                    nw = nw.max(100);
                                    let new_h = nh.max(100);
                                    ny += bbox.size.h - new_h; nh = new_h;
                                }
                                ResizeEdge::TopLeft => {
                                    let new_w = nw.max(100);
                                    nx += bbox.size.w - new_w; nw = new_w;
                                    let new_h = nh.max(100);
                                    ny += bbox.size.h - new_h; nh = new_h;
                                }
                            }
                            self.space.relocate_element(&elem, (nx, ny));
                            if let Some(xdg_toplevel) = window.toplevel() {
                                xdg_toplevel.with_pending_state(|state| state.size = Some((nw, nh).into()));
                                xdg_toplevel.send_configure();
                            }
                        }
                    }
                }
                self.gesture_origin = (point.x, point.y);
                self.render_all();
            }
            self.last_seat_dispatch = format!("gesture_move id={} target={:?}", id, target);
            true
        } else {
            false
        }
    }

    fn dispatch_touch_up_gesture(&mut self, id: i32) -> bool {
        if self.gesture_target.is_some() {
            self.gesture_target = None;
            self.gesture_surface = None;
            self.active_touch_ids.remove(&id);
            self.render_all();
            self.last_seat_dispatch = format!("gesture_touch_up released id={}", id);
            true
        } else {
            false
        }
    }

    fn handle_trackpad_down(&mut self, id: i32, point: &TouchPoint) {
        self.trackpad_anchor = Some((point.x, point.y));
        if self.active_touch_ids.is_empty() {
            self.trackpad_moved = false;
            self.trackpad_dragging = false;
            self.trackpad_hold_start_ms = engine_timing::now_ms_u32();
        }
        self.active_touch_ids.insert(id);
        // Do NOT warp the pointer to the touch position. Trackpad should
        // only send relative motion; the cursor stays where it was.
        // Just ensure seat/keyboard focus is established on some surface.
        if self.focused_surface.is_none() {
            self.apply_forced_focus("trackpad_down");
        }
        self.last_seat_dispatch = format!("trackpad_down id={} x={:.0} y={:.0}", id, point.x, point.y);
    }

    fn handle_trackpad_move(&mut self, id: i32, point: &TouchPoint) {
        let Some((last_x, last_y)) = self.trackpad_anchor else {
            self.trackpad_anchor = Some((point.x, point.y));
            return;
        };

        let raw_dx = point.x - last_x;
        let raw_dy = point.y - last_y;
        self.trackpad_anchor = Some((point.x, point.y));

        if raw_dx.abs() < 0.5 && raw_dy.abs() < 0.5 {
            return;
        }

        self.trackpad_moved = true;
        let p = self.pointer.clone();

        // Long-press detection: if finger held still > 350ms, enter drag mode.
        // Drag mode keeps left button pressed so subsequent motion acts as
        // click-and-drag (text selection, window move via titlebar).
        if !self.trackpad_dragging {
            let hold_ms = engine_timing::now_ms_u32() - self.trackpad_hold_start_ms;
            if hold_ms > 350 {
                self.trackpad_dragging = true;
                self.trackpad_tap_fingers.clear();
                p.button(self, &ButtonEvent {
                    serial: SERIAL_COUNTER.next_serial(),
                    time: engine_timing::now_ms_u32(),
                    button: 0x110,
                    state: ButtonState::Pressed,
                });
                p.frame(self);
                engine_timing::emit_hybrid_trace("Trackpad long-press → drag mode (button held)".to_string());
                self.last_seat_dispatch = "trackpad_drag_start".into();
            }
        }

        // Apply sensitivity and acceleration.
        // Reduced base sensitivity for precise control.
        // Base multiplier 1.5x, acceleration adds up to 3x for fast swipes.
        let speed = (raw_dx * raw_dx + raw_dy * raw_dy).sqrt();
        let accel = 1.0 + (speed / 150.0).min(2.0);
        let s = self.relative_sensitivity;
        let dx = raw_dx * 1.5 * accel * s;
        let dy = raw_dy * 1.5 * accel * s;

        // Clamp to screen resolution, NOT physical_size (which is mm).
        let (sw, sh) = self.screen_size;
        let current = p.current_location();
        let new_location = Point::<f64, Logical>::from((
            (current.x + dx as f64).clamp(0.0, sw as f64),
            (current.y + dy as f64).clamp(0.0, sh as f64),
        ));

        let pointer_focus = self.focused_surface.as_ref().map(|s| {
            let origin = self.wl_to_window.get(s)
                .and_then(|w| self.space.element_location(&WindowElement(w.clone())))
                .map(|loc| (loc.x as f64, loc.y as f64).into())
                .unwrap_or_else(|| (0.0, 0.0).into());
            (s.clone(), origin)
        });

        p.motion(self, pointer_focus, &PointerMotionEvent {
            location: new_location,
            serial: SERIAL_COUNTER.next_serial(),
            time: engine_timing::now_ms_u32(),
        });

        // Also send relative_motion for relative pointer protocol clients
        let cfocus = self.focused_surface.as_ref().map(|s| (s.clone(), (0.0, 0.0).into()));
        p.relative_motion(self, cfocus, &RelativeMotionEvent {
            delta: (dx as f64, dy as f64).into(),
            delta_unaccel: (dx as f64, dy as f64).into(),
            utime: (engine_timing::now_ms_u32() as u64) * 1000,
        });

        p.frame(self);

        engine_timing::emit_hybrid_trace(format!(
            "Trackpad {} id={} dx={:.1} dy={:.1} speed={:.0}",
            if self.trackpad_dragging { "drag_move" } else { "relative_move" },
            id, dx, dy, speed
        ));
        self.last_seat_dispatch = format!("trackpad_{} id={} dx={:.0} dy={:.0}",
            if self.trackpad_dragging { "drag" } else { "move" }, id, dx, dy);
    }

    fn handle_trackpad_up(&mut self, id: i32) {
        let was_moving = self.trackpad_moved;
        let was_dragging = self.trackpad_dragging;
        self.trackpad_anchor = None;
        self.trackpad_moved = false;
        self.trackpad_dragging = false;
        self.active_touch_ids.remove(&id);

        if was_dragging {
            // Release held button from drag mode
            let p = self.pointer.clone();
            p.button(self, &ButtonEvent {
                serial: SERIAL_COUNTER.next_serial(),
                time: engine_timing::now_ms_u32(),
                button: 0x110,
                state: ButtonState::Released,
            });
            p.frame(self);
            self.trackpad_tap_fingers.clear();
            engine_timing::emit_hybrid_trace("Trackpad drag→release".to_string());
            self.last_seat_dispatch = "trackpad_drag_end".into();
        } else if !was_moving {
            self.trackpad_tap_fingers.push(id);
            if self.active_touch_ids.is_empty() {
                let tap_count = self.trackpad_tap_fingers.len();
                self.trackpad_tap_fingers.clear();
                let p = self.pointer.clone();

                // Differentiate quick tap vs long-press:
                //   Quick tap (< 250ms): tap → left button click
                //   Long press (>= 400ms): hold+lift → right button click
                let hold_ms = engine_timing::now_ms_u32() - self.trackpad_hold_start_ms;
                let is_long_press = tap_count == 1 && hold_ms >= 400;

                let click_time = engine_timing::now_ms_u32();
                let button = if is_long_press || tap_count >= 2 { 0x111 } else { 0x110 };
                p.button(self, &ButtonEvent {
                    serial: SERIAL_COUNTER.next_serial(),
                    time: click_time,
                    button,
                    state: ButtonState::Pressed,
                });
                p.frame(self);
                p.button(self, &ButtonEvent {
                    serial: SERIAL_COUNTER.next_serial(),
                    time: click_time,
                    button,
                    state: ButtonState::Released,
                });
                p.frame(self);
                if tap_count >= 2 {
                    engine_timing::emit_hybrid_trace("Trackpad two-finger tap→right-click".to_string());
                    self.last_seat_dispatch = "trackpad_two_finger_tap".into();
                } else if is_long_press {
                    engine_timing::emit_hybrid_trace("Trackpad long-press→right-click".to_string());
                    self.last_seat_dispatch = "trackpad_long_press".into();
                } else {
                    engine_timing::emit_hybrid_trace(format!("Trackpad tap→click id={}", id));
                    self.last_seat_dispatch = format!("trackpad_tap id={}", id);
                }
            }
        } else {
            self.trackpad_tap_fingers.clear();
            self.last_seat_dispatch = format!("trackpad_up id={}", id);
        }
    }

    fn handle_absolute_pointer_down(&mut self, id: i32, point: &TouchPoint) {
        if self.primary_touch_id.is_some() {
            return;
        }
        self.primary_touch_id = Some(id);
        let forced_focus = self.apply_forced_focus_at("abs_mouse_down", point.x, point.y);
        let pointer = self.pointer.clone();
        let location = engine_timing::point_from_xy(point.x, point.y);
        let pointer_focus = forced_focus.as_ref().map(|s| {
            let origin = self.wl_to_window.get(s)
                .and_then(|w| self.space.element_location(&WindowElement(w.clone())))
                .map(|loc| (loc.x as f64, loc.y as f64).into())
                .unwrap_or_else(|| (0.0, 0.0).into());
            (s.clone(), origin)
        });
        let motion_time = engine_timing::now_ms_u32();
        pointer.motion(self, pointer_focus, &PointerMotionEvent {
            location,
            serial: SERIAL_COUNTER.next_serial(),
            time: motion_time,
        });
        pointer.frame(self);
        let press_time = engine_timing::now_ms_u32();
        pointer.button(self, &ButtonEvent {
            serial: SERIAL_COUNTER.next_serial(),
            time: press_time,
            button: 0x110,
            state: ButtonState::Pressed,
        });
        pointer.frame(self);
        engine_timing::emit_hybrid_trace(format!(
            "AbsoluteMouse pointer_down id={} x={:.1} y={:.1} motion_t={} press_t={}",
            id, point.x, point.y, motion_time, press_time
        ));
        self.last_seat_dispatch = format!("abs_mouse_down id={} x={:.0} y={:.0}", id, point.x, point.y);
        self.active_touch_ids.insert(id);
    }

    fn handle_absolute_pointer_move(&mut self, id: i32, point: &TouchPoint, focus: &Option<WlSurface>) {
        if self.primary_touch_id != Some(id) {
            return;
        }
        let pointer = self.pointer.clone();
        let location = engine_timing::point_from_xy(point.x, point.y);
        let surface_origin: Point<f64, Logical> = focus.as_ref()
            .and_then(|s| self.wl_to_window.get(s))
            .and_then(|w| self.space.element_location(&WindowElement(w.clone())))
            .map(|loc| (loc.x as f64, loc.y as f64).into())
            .unwrap_or_else(|| (0.0, 0.0).into());
        let pointer_focus = focus.as_ref().map(|s| (s.clone(), surface_origin));
        let motion_time = engine_timing::now_ms_u32();
        pointer.motion(self, pointer_focus, &PointerMotionEvent {
            location,
            serial: SERIAL_COUNTER.next_serial(),
            time: motion_time,
        });
        pointer.frame(self);
        engine_timing::emit_hybrid_trace(format!(
            "AbsoluteMouse pointer_move id={} x={:.1} y={:.1}",
            id, point.x, point.y
        ));
        self.last_seat_dispatch = format!("abs_mouse_move id={} x={:.0} y={:.0}", id, point.x, point.y);
    }

    fn handle_absolute_pointer_up(&mut self, id: i32) {
        if self.primary_touch_id != Some(id) {
            return;
        }
        let pointer = self.pointer.clone();
        let release_time = engine_timing::now_ms_u32();
        pointer.button(self, &ButtonEvent {
            serial: SERIAL_COUNTER.next_serial(),
            time: release_time,
            button: 0x110,
            state: ButtonState::Released,
        });
        pointer.frame(self);
        engine_timing::emit_hybrid_trace(format!(
            "AbsoluteMouse pointer_up id={}", id
        ));
        self.primary_touch_id = None;
        self.last_seat_dispatch = format!("abs_mouse_up id={}", id);
    }

    fn handle_touch_down(&mut self, id: i32, point: &TouchPoint, focus: &Option<WlSurface>) {
        let touch = self.touch.clone();
        let location = engine_timing::point_from_xy(point.x, point.y);
        let surface_origin: Point<f64, Logical> = focus.as_ref()
            .and_then(|s| self.wl_to_window.get(s))
            .and_then(|w| self.space.element_location(&WindowElement(w.clone())))
            .map(|loc| (loc.x as f64, loc.y as f64).into())
            .unwrap_or_else(|| (0.0, 0.0).into());
        let touch_focus = self.focused_surface.clone()
            .or_else(|| self.space.elements().rev()
                .filter_map(|elem| elem.0.wl_surface().map(|s| s.as_ref().clone()))
                .find(|s| s.is_alive()))
            .as_ref()
            .map(|s| (s.clone(), surface_origin));
        let down_time = engine_timing::now_ms_u32();
        touch.down(self, touch_focus, &DownEvent {
            slot: engine_timing::touch_slot_from_id(id),
            location,
            serial: SERIAL_COUNTER.next_serial(),
            time: down_time,
        });
        touch.frame(self);
        engine_timing::emit_hybrid_trace(format!(
            "TouchOnly touch_down id={} x={:.1} y={:.1}", id, point.x, point.y
        ));
        self.last_seat_dispatch = format!("touch_down id={} x={:.0} y={:.0}", id, point.x, point.y);
        self.active_touch_ids.insert(id);
        if self.active_touch_ids.len() >= 3 {
            self.swipe_cycle_armed = true;
        }
        self.swipe_starts.insert(id, (point.x, point.y, point.x, point.y));
    }

    fn handle_touch_move(&mut self, id: i32, point: &TouchPoint, focus: &Option<WlSurface>) {
        let touch = self.touch.clone();
        let location = engine_timing::point_from_xy(point.x, point.y);
        let surface_origin: Point<f64, Logical> = focus.as_ref()
            .and_then(|s| self.wl_to_window.get(s))
            .and_then(|w| self.space.element_location(&WindowElement(w.clone())))
            .map(|loc| (loc.x as f64, loc.y as f64).into())
            .unwrap_or_else(|| (0.0, 0.0).into());
        let touch_focus = focus.as_ref().map(|s| (s.clone(), surface_origin));
        let move_time = engine_timing::now_ms_u32();
        touch.motion(self, touch_focus, &TouchMotionEvent {
            slot: engine_timing::touch_slot_from_id(id),
            location,
            time: move_time,
        });
        touch.frame(self);
        engine_timing::emit_hybrid_trace(format!(
            "TouchOnly touch_move id={} x={:.1} y={:.1}", id, point.x, point.y
        ));
        self.last_seat_dispatch = format!("touch_move id={} x={:.0} y={:.0}", id, point.x, point.y);
        if let Some(entry) = self.swipe_starts.get_mut(&id) {
            entry.2 = point.x;
            entry.3 = point.y;
        }
    }

    fn handle_touch_up(&mut self, id: i32) {
        let touch = self.touch.clone();
        let up_time = engine_timing::now_ms_u32();
        touch.up(self, &UpEvent {
            slot: engine_timing::touch_slot_from_id(id),
            serial: SERIAL_COUNTER.next_serial(),
            time: up_time,
        });
        touch.frame(self);
        engine_timing::emit_hybrid_trace(format!(
            "TouchOnly touch_up id={}", id
        ));

        // Swipe-to-cycle-window gesture
        if let Some((start_x, start_y, last_x, last_y)) = self.swipe_starts.remove(&id) {
            let (screen_w, screen_h) = self.usable_screen_size();
            let dx = last_x - start_x;
            let dy = last_y - start_y;
            let now = engine_timing::now_ms_u32();
            let cooldown_ready = self.last_window_cycle_ms == 0
                || now.wrapping_sub(self.last_window_cycle_ms) >= self.window_cycle_cooldown_ms;
            if self.swipe_cycle_armed && cooldown_ready
                && dx.abs() > screen_w as f32 * 0.35
                && dy.abs() < screen_h as f32 * 0.18
            {
                let direction = if dx > 0.0 { 1 } else { -1 };
                self.cycle_window_focus(direction);
                self.last_window_cycle_ms = now;
            }
        }
        self.active_touch_ids.remove(&id);
        if self.active_touch_ids.is_empty() {
            self.swipe_cycle_armed = false;
        }
        self.last_seat_dispatch = format!("touch_up id={}", id);
    }

    pub(crate) fn inject_trackpad_relative(&mut self, dx: f32, dy: f32, time: u32) {
        if self.current_input_mode != WinlandInputMode::Trackpad {
            return;
        }
        // Ensure pointer has a target surface for the cursor to appear on.
        if self.focused_surface.is_none() {
            self.apply_forced_focus("trackpad_rel");
        }
        let p = self.pointer.clone();

        // Clamp to screen resolution, NOT physical_size (which is mm).
        let (sw, sh) = self.screen_size;
        let current = p.current_location();
        let new_location = Point::<f64, Logical>::from((
            (current.x + dx as f64).clamp(0.0, sw as f64),
            (current.y + dy as f64).clamp(0.0, sh as f64),
        ));

        let pointer_focus = self.focused_surface.as_ref().map(|s| {
            let origin = self.wl_to_window.get(s)
                .and_then(|w| self.space.element_location(&WindowElement(w.clone())))
                .map(|loc| (loc.x as f64, loc.y as f64).into())
                .unwrap_or_else(|| (0.0, 0.0).into());
            (s.clone(), origin)
        });

        p.motion(self, pointer_focus, &PointerMotionEvent {
            location: new_location,
            serial: SERIAL_COUNTER.next_serial(),
            time,
        });

        let cfocus = self.focused_surface.as_ref().map(|s| (s.clone(), (0.0, 0.0).into()));
        p.relative_motion(self, cfocus, &RelativeMotionEvent {
            delta: (dx as f64, dy as f64).into(),
            delta_unaccel: (dx as f64, dy as f64).into(),
            utime: (time as u64) * 1000,
        });
        p.frame(self);
        self.injected_events += 1;
        engine_timing::emit_hybrid_trace(format!(
            "Trackpad relative_motion dx={:.1} dy={:.1} t={}", dx, dy, time
        ));
        self.last_seat_dispatch = format!("trackpad_rel dx={:.0} dy={:.0}", dx, dy);
    }

    pub(crate) fn inject_trackpad_click(&mut self, state: i32, button: i32, time: u32) {
        if self.current_input_mode != WinlandInputMode::Trackpad {
            return;
        }
        let p = self.pointer.clone();
        let button_state = if state == 1 { ButtonState::Pressed } else { ButtonState::Released };
        p.button(self, &ButtonEvent {
            serial: SERIAL_COUNTER.next_serial(),
            time,
            button: button as u32,
            state: button_state,
        });
        p.frame(self);
        self.injected_events += 1;
        engine_timing::emit_hybrid_trace(format!(
            "Trackpad click state={} btn=0x{:x} t={}", state, button, time
        ));
        self.last_seat_dispatch = format!("trackpad_click state={} btn=0x{:x}", state, button);

        // Two-finger right-click: a finger is held while we inject a right-click press.
        // Mark trackpad as moved so handle_trackpad_up skips tap-click generation
        // when that finger lifts, preventing the tap from dismissing the context menu.
        if state == 1 && button == 0x111 && !self.active_touch_ids.is_empty() {
            self.trackpad_moved = true;
        }
    }

    pub(crate) fn inject_routed_event(&mut self, event: &RoutedInputEvent) {
        if !engine_timing::is_rendering_active() {
            engine_timing::emit_hybrid_trace(format!(
                "inject_routed_event: dropped rendering-inactive event={:?}", event
            ));
            return;
        }

        self.injected_events += 1;

        if self.space.elements().next().is_none()
            && self.unmanaged_surfaces.is_empty()
            && self.focused_surface.is_none()
        {
            engine_timing::emit_hybrid_trace(format!(
                "inject_routed_event: dropped no-windows event={:?}", event
            ));
            return;
        }

        let focus = self.focused_surface.clone()
            .or_else(|| {
                self.space.elements().rev()
                    .filter_map(|elem| elem.0.wl_surface().map(|s| s.as_ref().clone()))
                    .find(|s| s.is_alive())
            });
        let has_focus = focus.is_some();
        let mode = self.current_input_mode;
        let is_xwayland = focus.as_ref().map(is_xwayland_surface).unwrap_or(false);

        match mode {
            WinlandInputMode::Touch => {
                match event {
                    RoutedInputEvent::TouchDown { id, point } => {
                        self.dispatch_touch_down(*id, point);
                        if self.gesture_target.is_some() || self.last_seat_dispatch.contains("_btn ") {
                            return;
                        }
                        if is_xwayland {
                            self.handle_absolute_pointer_down(*id, point);
                        }
                        self.handle_touch_down(*id, point, &focus);
                    }
                    RoutedInputEvent::TouchMove { id, point } => {
                        if self.dispatch_touch_move_gesture(*id, point) {
                            return;
                        }
                        if is_xwayland {
                            self.handle_absolute_pointer_move(*id, point, &focus);
                        }
                        self.handle_touch_move(*id, point, &focus);
                    }
                    RoutedInputEvent::TouchUp { id } => {
                        if self.dispatch_touch_up_gesture(*id) {
                            return;
                        }
                        if is_xwayland {
                            self.handle_absolute_pointer_up(*id);
                        }
                        self.handle_touch_up(*id);
                    }
                    RoutedInputEvent::TouchCancel { .. } => {
                        let t = self.touch.clone();
                        t.cancel(self);
                        t.frame(self);
                        self.swipe_starts.clear();
                        self.active_touch_ids.clear();
                        self.swipe_cycle_armed = false;
                        self.last_seat_dispatch = format!("touch_cancel touch focus={}", has_focus);
                    }
                    _ => {}
                }
            }
            WinlandInputMode::Trackpad => {
                match event {
                    RoutedInputEvent::TouchDown { id, point } => {
                        self.handle_trackpad_down(*id, point);
                    }
                    RoutedInputEvent::TouchMove { id, point } => {
                        self.handle_trackpad_move(*id, point);
                    }
                    RoutedInputEvent::TouchUp { id } => {
                        self.handle_trackpad_up(*id);
                    }
                    RoutedInputEvent::TouchCancel { .. } => {
                        self.trackpad_anchor = None;
                        self.trackpad_moved = false;
                        self.trackpad_dragging = false;
                        self.trackpad_tap_fingers.clear();
                        self.active_touch_ids.clear();
                        self.last_seat_dispatch = format!("touch_cancel trackpad focus={}", has_focus);
                    }
                    _ => {}
                }
            }
            WinlandInputMode::Mouse => {
                match event {
                    RoutedInputEvent::TouchDown { id, point } => {
                        self.handle_absolute_pointer_down(*id, point);
                    }
                    RoutedInputEvent::TouchMove { id, point } => {
                        if self.primary_touch_id == Some(*id) && self.focused_surface.is_some() {
                            use smithay::wayland::pointer_constraints::with_pointer_constraint;
                            let constrained = self.focused_surface.as_ref().map(|s| {
                                with_pointer_constraint::<Self, _, _>(s, &self.pointer, |c| c.is_some())
                            }).unwrap_or(false);
                            if constrained {
                                let raw_dx = point.x - self.gesture_origin.0;
                                let raw_dy = point.y - self.gesture_origin.1;
                                self.gesture_origin = (point.x, point.y);
                                let speed = (raw_dx * raw_dx + raw_dy * raw_dy).sqrt();
                                let accel = 1.0 + 0.3 * (speed / 300.0).min(2.0);
                                let s = self.relative_sensitivity;
                                let dx = raw_dx * s * accel;
                                let dy = raw_dy * s * accel;
                                let p = self.pointer.clone();
                                let cfocus = self.focused_surface.as_ref().map(|s| (s.clone(), (0.0, 0.0).into()));
                                p.relative_motion(self, cfocus, &RelativeMotionEvent {
                                    delta: (dx as f64, dy as f64).into(),
                                    delta_unaccel: (dx as f64, dy as f64).into(),
                                    utime: (engine_timing::now_ms_u32() as u64) * 1000,
                                });
                                p.frame(self);
                                engine_timing::emit_hybrid_trace(format!(
                                    "Mouse constrained_relative id={} dx={:.1} dy={:.1}", id, dx, dy
                                ));
                                self.last_seat_dispatch = format!("mouse_constrained_rel id={} dx={:.0} dy={:.0}", id, dx, dy);
                                return;
                            }
                        }
                        self.handle_absolute_pointer_move(*id, point, &focus);
                    }
                    RoutedInputEvent::TouchUp { id } => {
                        self.handle_absolute_pointer_up(*id);
                    }
                    RoutedInputEvent::TouchCancel { .. } => {
                        if let Some(primary) = self.primary_touch_id.take() {
                            let p = self.pointer.clone();
                            p.button(self, &ButtonEvent {
                                serial: SERIAL_COUNTER.next_serial(),
                                time: engine_timing::now_ms_u32(),
                                button: 0x110,
                                state: ButtonState::Released,
                            });
                            p.frame(self);
                            engine_timing::emit_hybrid_trace(format!(
                                "Mouse pointer_cancel primary={}", primary
                            ));
                        }
                        self.active_touch_ids.clear();
                        self.last_seat_dispatch = format!("touch_cancel mouse focus={}", has_focus);
                    }
                    _ => {}
                }
            }
        }

        // Non-touch events: handled identically for all modes
        match event {
            RoutedInputEvent::KeyDown { keycode } => {
                self.ensure_focus_for_non_pointer("key_down");

                if *keycode == 61 && self.android_modifiers.alt {
                    let dir = if self.android_modifiers.shift { -1 } else { 1 };
                    self.cycle_window_focus(dir);
                    self.last_seat_dispatch = if dir > 0 {
                        "window_switch alt+tab forward".to_string()
                    } else {
                        "window_switch alt+shift+tab backward".to_string()
                    };
                    return;
                }

                let scancode = super::keymap::android_keycode_to_xkb_scancode(*keycode);
                if scancode == 0 {
                    log::debug!(
                        "SmithayRuntime: dropped unknown Android keycode {}",
                        keycode
                    );
                    return;
                }

                let Some(keyboard) = self.keyboard.clone() else {
                    log::debug!(
                        "SmithayRuntime: dropping keydown because keyboard init is unavailable"
                    );
                    return;
                };

                if self.update_modifier_state_from_android_key(*keycode, true) {
                    self.sync_keyboard_modifiers("key_down");
                }

                keyboard.input(
                    self,
                    smithay::backend::input::Keycode::from(scancode),
                    KeyState::Pressed,
                    SERIAL_COUNTER.next_serial(),
                    engine_timing::now_ms_u32(),
                    |_data, _mods, _key| FilterResult::<()>::Forward,
                );
                self.last_seat_dispatch = format!("key_down sc={} focus={}", scancode, has_focus);
            }
            RoutedInputEvent::KeyUp { keycode } => {
                self.ensure_focus_for_non_pointer("key_up");
                let scancode = super::keymap::android_keycode_to_xkb_scancode(*keycode);
                if scancode == 0 {
                    log::debug!(
                        "SmithayRuntime: dropped unknown Android keycode {}",
                        keycode
                    );
                    return;
                }

                let Some(keyboard) = self.keyboard.clone() else {
                    log::debug!(
                        "SmithayRuntime: dropping keyup because keyboard init is unavailable"
                    );
                    return;
                };

                if self.update_modifier_state_from_android_key(*keycode, false) {
                    self.sync_keyboard_modifiers("key_up");
                }

                keyboard.input(
                    self,
                    smithay::backend::input::Keycode::from(scancode),
                    KeyState::Released,
                    SERIAL_COUNTER.next_serial(),
                    engine_timing::now_ms_u32(),
                    |_data, _mods, _key| FilterResult::<()>::Forward,
                );
                self.last_seat_dispatch = format!("key_up sc={} focus={}", scancode, has_focus);
            }
            RoutedInputEvent::TextCommit { text } => {
                self.ensure_focus_for_non_pointer("text_commit");
                if let Some(keyboard) = self.keyboard.clone() {
                    if keyboard.is_grabbed() {
                        log::debug!(
                            "TextCommit: skipping inject — keyboard is grabbed by IME client"
                        );
                        return;
                    }
                }
                self.inject_text_commit(text);
            }
            RoutedInputEvent::GestureScroll { dx, dy } => {
                self.ensure_focus_for_non_pointer("gesture_scroll");
                let sensitivity =
                    crate::android::command_channel::get_scroll_sensitivity() as f64;
                let pointer = self.pointer.clone();
                let axis = AxisFrame::new(engine_timing::now_ms_u32())
                    .source(AxisSource::Finger)
                    .value(Axis::Horizontal, -(*dx as f64) * sensitivity)
                    .value(Axis::Vertical, -(*dy as f64) * sensitivity);
                pointer.axis(self, axis);
                pointer.frame(self);
                self.last_seat_dispatch = format!(
                    "scroll dx={:.1} dy={:.1} focus={}",
                    dx, dy, has_focus
                );
            }
            _ => {}
        }

        engine_timing::emit_hybrid_trace(format!(
            "inject_routed_event mode={:?} event={:?}", mode, event
        ));
    }

    fn find_keycode_for_char(&self, ch: char) -> Option<(u32, bool)> {
        let target = xkbcommon::xkb::Keysym::from_char(ch);
        let keymap = &self.xkb_keymap.0;
        let min = keymap.min_keycode().raw();
        let max = keymap.max_keycode().raw();

        for raw in min..=max {
            let kc = xkbcommon::xkb::Keycode::new(raw);

            let syms = keymap.key_get_syms_by_level(kc, 0, 0);
            if syms.contains(&target) {
                return Some((raw, false));
            }

            let syms = keymap.key_get_syms_by_level(kc, 0, 1);
            if syms.contains(&target) {
                return Some((raw, true));
            }
        }

        None
    }

    pub(crate) fn inject_text_commit(&mut self, text: &str) {
        let modifier_active = self.android_modifiers.ctrl || self.android_modifiers.alt;
        for ch in text.chars() {
            let ch = if modifier_active {
                ch.to_ascii_lowercase()
            } else {
                ch
            };

            if let Some((scancode, with_shift)) = self.find_keycode_for_char(ch) {
                let suppress_shift = modifier_active && with_shift;

                log::debug!(
                    "inject_text_commit: char={:?} U+{:04X} scancode={} shift={} suppress={} ctrl={} alt={}",
                    ch, ch as u32, scancode, with_shift, suppress_shift,
                    self.android_modifiers.ctrl, self.android_modifiers.alt,
                );

                if with_shift && !suppress_shift {
                    self.inject_key_scancode(42 + 8, KeyState::Pressed);
                }

                self.inject_key_scancode(scancode, KeyState::Pressed);
                self.inject_key_scancode(scancode, KeyState::Released);

                if with_shift && !suppress_shift {
                    self.inject_key_scancode(42 + 8, KeyState::Released);
                }
            } else {
                log::warn!("inject_text_commit: unsupported char {:?} (U+{:04X}) — not in keymap", ch, ch as u32);
            }
        }
    }

    pub(crate) fn inject_key_scancode(&mut self, scancode: u32, state: KeyState) {
        let Some(keyboard) = self.keyboard.clone() else {
            log::debug!(
                "SmithayRuntime: dropping text commit because keyboard init is unavailable"
            );
            return;
        };
        keyboard.input(
            self,
            smithay::backend::input::Keycode::from(scancode),
            state,
            SERIAL_COUNTER.next_serial(),
            engine_timing::now_ms_u32(),
            |_data, _mods, _key| FilterResult::<()>::Forward,
        );
    }
}
