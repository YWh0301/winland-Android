use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use jni::JNIEnv;
use jni::objects::{JClass, JString};

static CLIPBOARD: OnceLock<Mutex<(String, u64)>> = OnceLock::new();

/// IME visibility flag, set from seat.rs background thread,
/// polled from Kotlin main thread.
static IME_VISIBLE: AtomicBool = AtomicBool::new(false);

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
