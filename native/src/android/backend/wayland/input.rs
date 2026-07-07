use std::collections::{HashMap, HashSet};
use std::time::Instant;

const TOUCH_THRESHOLD_PX: f32 = 10.0;
const LONG_PRESS_DURATION: std::time::Duration = std::time::Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TouchGestureState {
    Idle,
    Pending { start_x: f32, start_y: f32, start_time: Instant, id: i32 },
    Scroll { last_x: f32, last_y: f32, id: i32 },
    Armed { start_x: f32, start_y: f32, id: i32 },
}

#[derive(Debug, Clone, Copy)]
pub struct TouchPoint {
	pub x: f32,
	pub y: f32,
	pub x_norm: f32,
	pub y_norm: f32,
}

#[derive(Debug, Clone)]
pub enum RoutedInputEvent {
	TouchDown {
		id: i32,
		point: TouchPoint,
	},
	TouchMove {
		id: i32,
		point: TouchPoint,
	},
	TouchUp {
		id: i32,
	},
	TouchCancel {
		id: i32,
	},
	TouchClick {
		id: i32,
		point: TouchPoint,
	},
	TouchRightClick {
		id: i32,
		point: TouchPoint,
	},
	KeyDown {
		keycode: i32,
	},
	KeyUp {
		keycode: i32,
	},
	TextCommit {
		text: String,
	},
	GestureScroll {
		dx: f32,
		dy: f32,
		rx: f32,
		ry: f32,
	},
	GestureScrollEnd,
}

#[derive(Debug)]
pub struct InputRouter {
	active_touches: HashMap<i32, TouchPoint>,
	pressed_keys: HashSet<i32>,
	last_multi_touch_centroid: Option<(f32, f32)>,
    touch_gesture_state: TouchGestureState,
    last_surface_size: (i32, i32),
    was_armed: bool,
}

impl Default for InputRouter {
    fn default() -> Self {
        Self {
            active_touches: HashMap::new(),
            pressed_keys: HashSet::new(),
            last_multi_touch_centroid: None,
            touch_gesture_state: TouchGestureState::Idle,
            last_surface_size: (0, 0),
            was_armed: false,
        }
    }
}

impl InputRouter {
	pub fn route_touch(
		&mut self,
		action: i32,
		id: i32,
		x: f32,
		y: f32,
		width: i32,
		height: i32,
	) -> Vec<RoutedInputEvent> {
		let point = normalize_point(x, y, width, height);
		let mut events = Vec::with_capacity(2);
        self.last_surface_size = (width, height);

		match action {
			0 | 5 => {
				self.active_touches.insert(id, point);
                // First/only touch: start gesture state machine
                if self.active_touches.len() == 1 {
                    self.touch_gesture_state = TouchGestureState::Pending {
                        start_x: x,
                        start_y: y,
                        start_time: Instant::now(),
                        id,
                    };
                    // Defer event emission — wait for timer or finger up
                } else {
                    // Multi-touch: check if first finger was Armed (long-press ready)
                    let armed_data = if let TouchGestureState::Armed { start_x, start_y, id: first_id } = self.touch_gesture_state {
                        Some((start_x, start_y, first_id))
                    } else {
                        None
                    };
                    let was_armed = self.was_armed;
                    self.was_armed = false;
                    self.touch_gesture_state = TouchGestureState::Idle;
                    if let Some((sx, sy, _first_id)) = armed_data {
                        // Armed + second finger → right-click at first finger's position
                        let (w, h) = (self.last_surface_size.0.max(1), self.last_surface_size.1.max(1));
                        let p = TouchPoint {
                            x: sx, y: sy,
                            x_norm: (sx / w as f32).clamp(0.0, 1.0),
                            y_norm: (sy / h as f32).clamp(0.0, 1.0),
                        };
                        events.push(RoutedInputEvent::TouchRightClick { id, point: p });
                    } else if was_armed {
                        // Was armed, finger moved (selection drag), now second finger → right-click at second finger's position
                        events.push(RoutedInputEvent::TouchRightClick { id, point });
                    } else {
                        // Normal multi-touch: emit deferred TouchDowns + current
                        for (&fid, fp) in &self.active_touches {
                            if fid != id {
                                events.push(RoutedInputEvent::TouchDown { id: fid, point: *fp });
                            }
                        }
                        events.push(RoutedInputEvent::TouchDown { id, point });
                    }
                }
			}
			2 => {
				self.active_touches.insert(id, point);

                match self.touch_gesture_state {
                    TouchGestureState::Pending { start_x, start_y, id: sid, .. }
                        if sid == id =>
                    {
                        let dist = ((x - start_x).powi(2) + (y - start_y).powi(2)).sqrt();
                        if dist > TOUCH_THRESHOLD_PX {
                            // Threshold exceeded → Scroll
                            self.touch_gesture_state = TouchGestureState::Scroll {
                                last_x: x, last_y: y, id,
                            };
                        }
                    }
                    TouchGestureState::Scroll { last_x, last_y, id: sid }
                        if sid == id =>
                    {
                        let dx = x - last_x;
                        let dy = y - last_y;
                        self.touch_gesture_state = TouchGestureState::Scroll {
                            last_x: x, last_y: y, id,
                        };
                        if dx.abs() > 0.001 || dy.abs() > 0.001 {
                            events.push(RoutedInputEvent::GestureScroll { dx, dy, rx: x, ry: y });
                        }
                    }
                    TouchGestureState::Armed { start_x, start_y, id: sid }
                        if sid == id =>
                    {
                        // First move after long-press: emit deferred TouchDown → window drag
                        let (w, h) = (self.last_surface_size.0.max(1), self.last_surface_size.1.max(1));
                        let p = TouchPoint {
                            x: start_x, y: start_y,
                            x_norm: (start_x / w as f32).clamp(0.0, 1.0),
                            y_norm: (start_y / h as f32).clamp(0.0, 1.0),
                        };
                        events.push(RoutedInputEvent::TouchDown { id, point: p });
                        events.push(RoutedInputEvent::TouchMove { id, point });
                        self.touch_gesture_state = TouchGestureState::Idle;
                    }
                    _ => {
                        // Other fingers in multi-touch, or state machine is Idle
                        events.push(RoutedInputEvent::TouchMove { id, point });
                    }
                }

                // Old multi-touch centroid scroll (2+ fingers, kept for compatibility)
				if self.active_touches.len() >= 2 {
					if let Some((cx, cy)) = self.centroid() {
						if let Some((lx, ly)) = self.last_multi_touch_centroid {
							let dx = cx - lx;
							let dy = cy - ly;
							if dx.abs() > 0.001 || dy.abs() > 0.001 {
								let (rx, ry) = self.centroid_raw().unwrap_or((0.0, 0.0));
								events.push(RoutedInputEvent::GestureScroll { dx, dy, rx, ry });
							}
						}
						self.last_multi_touch_centroid = Some((cx, cy));
					}
				} else {
					if self.last_multi_touch_centroid.take().is_some() {
						events.push(RoutedInputEvent::GestureScrollEnd);
					}
				}
			}
			1 | 6 => {
				self.active_touches.remove(&id);
                self.was_armed = false;

                match self.touch_gesture_state {
                    TouchGestureState::Pending { start_x, start_y, id: sid, .. }
                        if sid == id =>
                    {
                        // Tap: finger lifted with minimal/no movement
                        self.touch_gesture_state = TouchGestureState::Idle;
                        let (w, h) = (self.last_surface_size.0.max(1), self.last_surface_size.1.max(1));
                        let p = TouchPoint {
                            x: start_x, y: start_y,
                            x_norm: (start_x / w as f32).clamp(0.0, 1.0),
                            y_norm: (start_y / h as f32).clamp(0.0, 1.0),
                        };
                        events.push(RoutedInputEvent::TouchClick { id, point: p });
                    }
                    TouchGestureState::Scroll { id: sid, .. }
                        if sid == id =>
                    {
                        self.touch_gesture_state = TouchGestureState::Idle;
                        events.push(RoutedInputEvent::GestureScrollEnd);
                    }
                    TouchGestureState::Armed { id: sid, .. }
                        if sid == id =>
                    {
                        self.touch_gesture_state = TouchGestureState::Idle;
                        // swallow — long-press with no follow-up action
                    }
                    _ => {
                        events.push(RoutedInputEvent::TouchUp { id });
                    }
                }

				if self.active_touches.len() < 2 {
					if self.last_multi_touch_centroid.take().is_some() {
						events.push(RoutedInputEvent::GestureScrollEnd);
					}
				}
			}
			3 => {
				self.active_touches.remove(&id);
                self.was_armed = false;
                self.touch_gesture_state = TouchGestureState::Idle;
                self.last_multi_touch_centroid.take();
				events.push(RoutedInputEvent::TouchCancel { id });
			}
			_ => {
				self.active_touches.insert(id, point);
				events.push(RoutedInputEvent::TouchMove { id, point });
			}
		}

		events
	}

    /// Check if the Pending timer has expired (long-press).
    /// Must be called once per compositor loop iteration.
    /// Transitions to Armed state — waiting for window-drag or second finger.
    /// Returns None: the state change alone carries the information; next
    /// TouchMove (for drag) or multi-touch (for right-click) triggers emission.
    pub fn poll_timer(&mut self) -> Option<RoutedInputEvent> {
        if let TouchGestureState::Pending { start_x, start_y, start_time, id } = self.touch_gesture_state {
            if start_time.elapsed() >= LONG_PRESS_DURATION && self.active_touches.len() == 1 {
                self.touch_gesture_state = TouchGestureState::Armed { start_x, start_y, id };
                self.was_armed = true;
            }
        }
        None
    }

	fn centroid(&self) -> Option<(f32, f32)> {
		if self.active_touches.is_empty() {
			return None;
		}

		let mut sx = 0.0_f32;
		let mut sy = 0.0_f32;
		for p in self.active_touches.values() {
			sx += p.x_norm;
			sy += p.y_norm;
		}

		let c = self.active_touches.len() as f32;
		Some((sx / c, sy / c))
	}

	fn centroid_raw(&self) -> Option<(f32, f32)> {
		if self.active_touches.is_empty() {
			return None;
		}

		let mut sx = 0.0_f32;
		let mut sy = 0.0_f32;
		for p in self.active_touches.values() {
			sx += p.x;
			sy += p.y;
		}

		let c = self.active_touches.len() as f32;
		Some((sx / c, sy / c))
	}

	pub fn route_key(&mut self, keycode: i32, is_down: bool) -> RoutedInputEvent {
		if is_down {
			self.pressed_keys.insert(keycode);
			RoutedInputEvent::KeyDown { keycode }
		} else {
			self.pressed_keys.remove(&keycode);
			RoutedInputEvent::KeyUp { keycode }
		}
	}

	pub fn active_touch_count(&self) -> usize {
		self.active_touches.len()
	}

    pub fn pressed_key_count(&self) -> usize {
        self.pressed_keys.len()
    }

    pub fn clear(&mut self) {
        self.active_touches.clear();
        self.pressed_keys.clear();
        self.last_multi_touch_centroid = None;
        self.touch_gesture_state = TouchGestureState::Idle;
    }
}

fn normalize_point(x: f32, y: f32, width: i32, height: i32) -> TouchPoint {
	let width_f = if width <= 0 { 1.0 } else { width as f32 };
	let height_f = if height <= 0 { 1.0 } else { height as f32 };

	TouchPoint {
		x,
		y,
		x_norm: (x / width_f).clamp(0.0, 1.0),
		y_norm: (y / height_f).clamp(0.0, 1.0),
	}
}
