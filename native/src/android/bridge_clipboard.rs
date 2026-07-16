use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::jlong;

static CLIPBOARD: OnceLock<Mutex<(String, u64)>> = OnceLock::new();

/// IME visibility flag, set from seat.rs background thread,
/// polled from Kotlin main thread.
static IME_VISIBLE: AtomicBool = AtomicBool::new(false);
static OUTER_CURSOR: OnceLock<Mutex<(u64, i32, i32, bool)>> = OnceLock::new();
static OUTER_CURSOR_SCALE_BITS: AtomicU32 = AtomicU32::new(1.0f32.to_bits());

fn outer_cursor() -> &'static Mutex<(u64, i32, i32, bool)> {
    OUTER_CURSOR.get_or_init(|| Mutex::new((0, 0, 0, false)))
}

pub fn publish_outer_cursor_position(logical_x: f64, logical_y: f64, visible: bool) {
    let scale = f32::from_bits(OUTER_CURSOR_SCALE_BITS.load(Ordering::Relaxed)) as f64;
    let mut state = outer_cursor().lock().unwrap();
    state.0 = state.0.wrapping_add(1).max(1);
    state.1 = (logical_x * scale).round() as i32;
    state.2 = (logical_y * scale).round() as i32;
    state.3 = visible;
}

pub fn set_ime_visible(show: bool) {
    IME_VISIBLE.store(show, Ordering::Relaxed);
}

pub fn is_ime_visible() -> bool {
    IME_VISIBLE.load(Ordering::Relaxed)
}

fn get_clipboard() -> &'static Mutex<(String, u64)> {
    CLIPBOARD.get_or_init(|| Mutex::new((String::new(), 0)))
}

pub fn set_clipboard_text(text: &str) {
    let mut guard = get_clipboard().lock().unwrap();
    guard.0 = text.to_string();
    guard.1 = guard.1.wrapping_add(1);
}

pub fn get_clipboard_with_generation() -> (String, u64) {
    let guard = get_clipboard().lock().unwrap();
    guard.clone()
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_pollWaylandClipboard<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
) -> jni::sys::jstring {
    let text = get_clipboard().lock().unwrap().0.clone();
    env.new_string(&text).unwrap().into_raw()
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_getWaylandClipboardGen(
    _env: JNIEnv,
    _class: JClass,
) -> jni::sys::jlong {
    get_clipboard().lock().unwrap().1 as i64
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_pollImeVisible(
    _env: JNIEnv,
    _class: JClass,
) -> jni::sys::jboolean {
    jni::sys::jboolean::from(is_ime_visible())
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_setOuterCursorScale(
    _env: JNIEnv,
    _class: JClass,
    value: jni::sys::jfloat,
) {
    let value = if value.is_finite() { value.clamp(1.0, 8.0) } else { 1.0 };
    OUTER_CURSOR_SCALE_BITS.store(value.to_bits(), Ordering::Relaxed);
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_pollOuterCursorState(
    env: JNIEnv,
    _class: JClass,
) -> jni::sys::jlongArray {
    let state = *outer_cursor().lock().unwrap();
    let values: [jlong; 4] = [state.0 as jlong, state.1 as jlong, state.2 as jlong, state.3 as jlong];
    let Ok(array) = env.new_long_array(values.len() as i32) else { return std::ptr::null_mut(); };
    if env.set_long_array_region(&array, 0, &values).is_err() { return std::ptr::null_mut(); }
    array.into_raw()
}

#[no_mangle]
/// # Safety
///
/// Called from the JVM when the Android clipboard changes (Wayland→Android feedback or
/// explicit Android→Wayland push). Routes text into the Smithay data-device selection
/// so Wayland clients can paste from it.
pub unsafe extern "system" fn Java_com_winland_server_NativeBridge_updateClipboard(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) {
    let input: String = env.get_string(&text).expect("Couldn't get java string!").into();
    log::info!("Bridge: updateClipboard Android->Wayland len={}", input.len());
    #[cfg(feature = "smithay_android")]
    crate::android::command_channel::send_command(crate::android::command_channel::JniCommand::UpdateClipboard { text: input });
}

#[no_mangle]
/// # Safety
///
/// Called from the JVM when the Android clipboard changes.
/// Injects text as a server-side Wayland `wl_data_device` selection so clients can paste.
pub unsafe extern "system" fn Java_com_winland_server_NativeBridge_sendClipboardTextToWayland(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) {
    let input: String = env.get_string(&text).expect("Couldn't get java string!").into();
    log::info!("Bridge: sendClipboardTextToWayland len={}", input.len());
    #[cfg(feature = "smithay_android")]
    crate::android::command_channel::send_command(crate::android::command_channel::JniCommand::UpdateClipboard { text: input });
}
