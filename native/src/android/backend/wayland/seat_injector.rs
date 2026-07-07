use std::sync::{RwLock, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use super::input::RoutedInputEvent;

#[derive(Debug, Default, Clone, Copy)]
pub struct SeatInjectionStats {
    pub touch_down: u64,
    pub touch_move: u64,
    pub touch_up: u64,
    pub touch_cancel: u64,
    pub touch_click: u64,
    pub touch_right_click: u64,
    pub key_down: u64,
    pub key_up: u64,
    pub text_commit: u64,
    pub gesture_scroll: u64,
    pub gesture_scroll_end: u64,
    pub last_event_time_ms: u64,
}

static INJECTION_STATS: OnceLock<RwLock<SeatInjectionStats>> = OnceLock::new();

fn stats() -> &'static RwLock<SeatInjectionStats> {
    INJECTION_STATS.get_or_init(|| RwLock::new(SeatInjectionStats::default()))
}

pub fn record_injection(event: &RoutedInputEvent) {
    let mut guard = match stats().write() {
        Ok(guard) => guard,
        Err(_) => {
            log::error!("SeatInjector: stats lock poisoned");
            return;
        }
    };

    guard.last_event_time_ms = now_ms();

    match event {
        RoutedInputEvent::TouchDown { .. } => {
            guard.touch_down += 1;
        }
        RoutedInputEvent::TouchMove { .. } => {
            guard.touch_move += 1;
        }
        RoutedInputEvent::TouchUp { .. } => {
            guard.touch_up += 1;
        }
        RoutedInputEvent::TouchCancel { .. } => {
            guard.touch_cancel += 1;
        }
        RoutedInputEvent::KeyDown { .. } => {
            guard.key_down += 1;
        }
        RoutedInputEvent::KeyUp { .. } => {
            guard.key_up += 1;
        }
        RoutedInputEvent::TextCommit { .. } => {
            guard.text_commit += 1;
        }
        RoutedInputEvent::GestureScroll { .. } => {
            guard.gesture_scroll += 1;
        }
        RoutedInputEvent::GestureScrollEnd => {
            guard.gesture_scroll_end += 1;
        }
        RoutedInputEvent::TouchClick { .. } => {
            guard.touch_click += 1;
        }
        RoutedInputEvent::TouchRightClick { .. } => {
            guard.touch_right_click += 1;
        }
    }
}

pub fn snapshot_stats() -> SeatInjectionStats {
    if let Ok(guard) = stats().read() {
        return guard.clone();
    }
    SeatInjectionStats::default()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
