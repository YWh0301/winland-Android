use jni::{JNIEnv, objects::{JClass, JObject, JString}};
use jni::sys::jboolean;
use crate::android::utils::context::ApplicationContext;
use std::panic::AssertUnwindSafe;
use std::ptr;
use std::sync::atomic::{AtomicPtr, Ordering};

static LAST_INIT_ERROR: AtomicPtr<String> = AtomicPtr::new(ptr::null_mut());

fn set_last_init_error(message: impl Into<String>) {
    let old = LAST_INIT_ERROR.swap(
        Box::into_raw(Box::new(message.into())),
        Ordering::SeqCst,
    );
    if !old.is_null() {
        unsafe { drop(Box::from_raw(old)); }
    }
}

fn clear_last_init_error() {
    let old = LAST_INIT_ERROR.swap(ptr::null_mut(), Ordering::SeqCst);
    if !old.is_null() {
        unsafe { drop(Box::from_raw(old)); }
    }
}

/// Get the ANativeWindow from a Surface JObject and send it to the compositor
/// thread for EGL initialization.
fn bind_native_window_on_compositor(_env: &mut JNIEnv, native_window: *mut std::ffi::c_void) -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let sent = crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::BindNativeWindow {
            native_window: crate::android::command_channel::NativeWindowPtr(native_window),
            response: tx,
        },
    );
    if !sent {
        unsafe { ndk_sys::ANativeWindow_release(native_window as *mut _); }
        return Err("Compositor channel not initialized".to_string());
    }
    rx.recv_timeout(std::time::Duration::from_secs(30))
        .map_err(|_| "Timeout waiting for EGL init on compositor thread".to_string())?
}

/// Send EnableShm command to the compositor thread.
fn enable_shm_on_compositor() {
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::EnableShm,
    );
}

#[no_mangle]
/// # Safety
///
/// This JNI entrypoint must be invoked by the JVM with a valid `JNIEnv`, class,
/// `Surface`, and activity object for the current thread.
pub unsafe extern "system" fn Java_com_winland_server_NativeBridge_initWaylandConnection(
    mut env: JNIEnv,
    _class: JClass,
    surface: JObject,
    activity: JObject,
    distro_id: JObject,
) -> jboolean {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("WinlandNative")
            .with_max_level(log::LevelFilter::Debug),
    );
    clear_last_init_error();

    let init_result = std::panic::catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        log::info!("JNI: initWaylandConnection called.");

        let distro_id: String = env
            .get_string(&JString::from(distro_id))
            .map_err(|error| format!("Failed to get distro_id string: {error}"))?
            .into();

        let activity_ref = env
            .new_global_ref(activity)
            .map_err(|error| format!("Failed to create global activity ref: {error}"))?;

        log::info!("JNI: Phase 1 - Building ApplicationContext...");
        ApplicationContext::build(&mut env, &activity_ref)
            .map_err(|e| format!("ApplicationContext buildup failed: {e}"))?;

        log::info!("JNI: Phase 2 - Starting compositor thread (distro={distro_id})...");
        crate::compositor::spawn(&distro_id)
            .map_err(|error| format!("Compositor spawn failed: {error}"))?;

        log::info!("JNI: Phase 3 - Binding ANativeWindow on compositor thread...");
        let native_window = ndk_sys::ANativeWindow_fromSurface(env.get_native_interface(), surface.as_raw());
        if native_window.is_null() {
            return Err("Failed to get ANativeWindow from surface".to_string());
        }
        bind_native_window_on_compositor(&mut env, native_window as *mut std::ffi::c_void)?;

        log::info!("JNI: Phase 4 - Enabling SHM on compositor thread...");
        enable_shm_on_compositor();

        log::info!("JNI: initWaylandConnection completed successfully.");
        Ok(())
    }));

    match init_result {
        Ok(Ok(())) => jboolean::from(true),
        Ok(Err(error)) => {
            set_last_init_error(error.clone());
            log::error!("JNI: {}", error);
            jboolean::from(false)
        }
        Err(_) => {
            let message = "initWaylandConnection panicked; compositor startup aborted safely".to_string();
            set_last_init_error(message.clone());
            log::error!("JNI: {}", message);
            jboolean::from(false)
        }
    }
}

#[no_mangle]
/// # Safety
///
/// This JNI entrypoint must be invoked by the JVM with a valid `JNIEnv` and
/// a Context-like object that provides getCacheDir/getFilesDir/getApplicationInfo.
pub unsafe extern "system" fn Java_com_winland_server_NativeBridge_ensureSocketRuntime(
    mut env: JNIEnv,
    _class: JClass,
    context: JObject,
    distro_id: JObject,
) -> jboolean {
    android_logger::init_once(
        android_logger::Config::default()
            .with_tag("WinlandNative")
            .with_max_level(log::LevelFilter::Debug),
    );

    let result = std::panic::catch_unwind(AssertUnwindSafe(|| -> Result<(), String> {
        let distro_id: String = env
            .get_string(&JString::from(distro_id))
            .map_err(|error| format!("Failed to get distro_id string: {error}"))?
            .into();

        let context_ref = env
            .new_global_ref(context)
            .map_err(|error| format!("Failed to create global context ref: {error}"))?;

        ApplicationContext::build(&mut env, &context_ref)
            .map_err(|e| format!("ApplicationContext buildup failed: {e}"))?;

        log::info!("JNI: ensureSocketRuntime started (distro={distro_id})");
        crate::compositor::spawn(&distro_id)
            .map_err(|error| format!("Compositor bootstrap failed: {error}"))?;

        log::info!("JNI: ensureSocketRuntime completed (Wayland socket should be available)");
        Ok(())
    }));

    match result {
        Ok(Ok(())) => jboolean::from(true),
        Ok(Err(error)) => {
            log::error!("JNI: ensureSocketRuntime failed: {}", error);
            jboolean::from(false)
        }
        Err(_) => {
            log::error!("JNI: ensureSocketRuntime panicked");
            jboolean::from(false)
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_releaseWaylandConnection(
    _env: JNIEnv,
    _class: JClass,
) {
    let (tx, rx) = std::sync::mpsc::channel();
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::ReleaseNativeWindow { response: tx },
    );
    let _ = rx.recv_timeout(std::time::Duration::from_secs(5));

    crate::compositor::stop();
}

pub(crate) fn get_latest_error() -> Option<String> {
    let ptr = LAST_INIT_ERROR.load(Ordering::SeqCst);
    if ptr.is_null() {
        None
    } else {
        unsafe { Some((*ptr).clone()) }
    }
}

/// JNI entry point for checking if Wayland clients are connected.
/// Returns 1 if at least one client is connected, 0 otherwise.
/// Reads a cached value updated by the compositor each frame (no channel round-trip).
#[no_mangle]
pub unsafe extern "system" fn Java_com_winland_server_NativeBridge_areClientsConnected(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    jboolean::from(crate::android::command_channel::are_clients_connected())
}

#[cfg(all(test, feature = "smithay_android"))]
mod tests {
    use super::*;

    #[test]
    fn latest_error_is_none_initially() {
        clear_last_init_error();
        assert_eq!(get_latest_error(), None);
    }

    #[test]
    fn set_and_get_error() {
        clear_last_init_error();
        assert_eq!(get_latest_error(), None);

        set_last_init_error("test error");
        assert_eq!(get_latest_error(), Some("test error".to_string()));
    }

    #[test]
    fn clear_removes_error() {
        clear_last_init_error();
        set_last_init_error("error");
        assert_eq!(get_latest_error(), Some("error".to_string()));

        clear_last_init_error();
        assert_eq!(get_latest_error(), None);
    }

    #[test]
    fn overwrite_replaces_error() {
        clear_last_init_error();
        set_last_init_error("first error");
        set_last_init_error("second error");
        assert_eq!(get_latest_error(), Some("second error".to_string()));
    }

    #[test]
    fn atomic_ptr_starts_null() {
        assert!(LAST_INIT_ERROR.load(Ordering::SeqCst).is_null());
    }

    #[test]
    fn set_error_non_empty() {
        clear_last_init_error();
        set_last_init_error("error with long message that exceeds typical length");
        let err = get_latest_error().unwrap();
        assert!(err.contains("typical length"));
    }

    #[test]
    fn clear_when_already_clear_is_safe() {
        clear_last_init_error();
        clear_last_init_error();
        clear_last_init_error();
        assert_eq!(get_latest_error(), None);
    }
}
