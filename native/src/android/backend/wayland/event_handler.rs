use crate::android::backend::smithay_backend::InputEvent;
use std::sync::{Mutex, OnceLock};

use super::input::{InputRouter, RoutedInputEvent};

pub fn handle() {
	flush_android_input_events();
}

static INPUT_ROUTER: OnceLock<Mutex<InputRouter>> = OnceLock::new();

fn input_router() -> &'static Mutex<InputRouter> {
	INPUT_ROUTER.get_or_init(|| Mutex::new(InputRouter::default()))
}

fn dispatch_routed_event(event: RoutedInputEvent) {
	super::seat_injector::inject_routed_event(&event);

	match event {
		RoutedInputEvent::TouchDown { id, point } => {
			log::debug!(
				"WaylandInput: touch_down id={} x={} y={} nx={} ny={}",
				id,
				point.x,
				point.y,
				point.x_norm,
				point.y_norm
			);
		}
		RoutedInputEvent::TouchMove { id, point } => {
			log::debug!(
				"WaylandInput: touch_move id={} x={} y={} nx={} ny={}",
				id,
				point.x,
				point.y,
				point.x_norm,
				point.y_norm
			);
		}
		RoutedInputEvent::TouchUp { id } => {
			log::debug!("WaylandInput: touch_up id={}", id);
		}
		RoutedInputEvent::TouchCancel { id } => {
			log::debug!("WaylandInput: touch_cancel id={}", id);
		}
		RoutedInputEvent::KeyDown { keycode } => {
			log::debug!("WaylandInput: key_down keycode={}", keycode);
		}
		RoutedInputEvent::KeyUp { keycode } => {
			log::debug!("WaylandInput: key_up keycode={}", keycode);
		}
		RoutedInputEvent::TextCommit { text } => {
			log::debug!("WaylandInput: text_commit len={}", text.len());
		}
		RoutedInputEvent::GestureScroll { dx, dy, rx, ry } => {
			log::debug!("WaylandInput: gesture_scroll dx={} dy={} rx={} ry={}", dx, dy, rx, ry);
		}
		RoutedInputEvent::GestureScrollEnd => {
			log::debug!("WaylandInput: gesture_scroll_end");
		}
	}
}

pub fn handle_android_input_event(event: &InputEvent) {
	let (width, height) = crate::android::backend::smithay_backend::surface_size();
	let mut router = match input_router().lock() {
		Ok(router) => router,
		Err(_) => {
			log::error!("WaylandInput: input router lock poisoned");
			return;
		}
	};

	let routed_events = match event {
		InputEvent::Touch { action, id, x, y } => router.route_touch(*action, *id, *x, *y, width, height),
		InputEvent::Key { keycode, is_down } => vec![router.route_key(*keycode, *is_down)],
		InputEvent::Text { text } => vec![RoutedInputEvent::TextCommit { text: text.clone() }],
	};

	for routed in routed_events {
		dispatch_routed_event(routed);
	}
}

pub fn flush_android_input_events() {
	let events = crate::android::backend::smithay_backend::take_pending_input();
	for event in &events {
		handle_android_input_event(event);
	}

	// Events are queued to SeatInjector above. The main compositor loop
	// drains them via flush_to_smithay_seat() on the render thread, so
	// we must NOT call it here — doing so would block the Android UI
	// thread on the runtime Mutex and cause ANR.

	if let Ok(router) = input_router().lock() {
		let stats = super::seat_injector::snapshot_stats();
		log::trace!(
			"WaylandInput: status active_touches={} pressed_keys={} injected_td={} injected_tm={} injected_tu={} injected_kd={} injected_ku={} injected_tx={}",
			router.active_touch_count(),
			router.pressed_key_count(),
			stats.touch_down,
			stats.touch_move,
			stats.touch_up,
			stats.key_down,
			stats.key_up,
			stats.text_commit
		);
	}
}
