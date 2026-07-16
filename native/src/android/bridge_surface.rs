use jni::{JNIEnv, objects::{JClass, JObject}};
use jni::sys::{jboolean, jfloat, jint};

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_onSurfaceChanged(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
    physical_width_mm: jint,
    physical_height_mm: jint,
) {
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::SetPhysicalSize {
            width_mm: physical_width_mm,
            height_mm: physical_height_mm,
        },
    );
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::SurfaceChanged {
            width,
            height,
            physical_width_mm,
            physical_height_mm,
        },
    );
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_setRefreshRate(
    _env: JNIEnv,
    _class: JClass,
    value: jfloat,
) {
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::SetRefreshRate { value },
    );
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_setResolution(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) {
    if width <= 0 || height <= 0 {
        log::warn!(
            "JNI: ignoring invalid setResolution request {}x{}",
            width,
            height
        );
        return;
    }

    // Use SetResolution (not SurfaceChanged) so the compositor distinguishes
    // user-initiated resolution changes from actual Android surface changes.
    // SurfaceChanged sets logical_size on first call; SetResolution only
    // updates surface_size (physical viewport).
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::SetResolution {
            width,
            height,
        },
    );
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_setYOffset(
    _env: JNIEnv,
    _class: JClass,
    y_offset: jint,
) {
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::SetYOffset { y_offset: y_offset.max(0) },
    );
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_setScale(
    _env: JNIEnv,
    _class: JClass,
    scale: jfloat,
) {
    if scale <= 0.0 {
        log::warn!("JNI: ignoring invalid setScale request {}", scale);
        return;
    }
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::SetScale { scale },
    );
}

#[no_mangle]
/// # Safety
///
/// Called from JNI when the Android Surface is recreated after surfaceDestroyed.
/// Extracts a new ANativeWindow from the Surface and sends BindNativeWindow
/// to the compositor thread to re-establish EGL context.
pub unsafe extern "system" fn Java_com_winland_server_NativeBridge_rebindSurface(
    env: JNIEnv,
    _class: JClass,
    surface: JObject,
) {
    log::info!("Bridge: rebindSurface called");
    let native_window = ndk_sys::ANativeWindow_fromSurface(env.get_native_interface(), surface.as_raw());
    if native_window.is_null() {
        log::error!("Bridge: rebindSurface - failed to get ANativeWindow from Surface");
        return;
    }
    let sent = crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::BindNativeWindow {
            native_window: crate::android::command_channel::NativeWindowPtr(native_window as *mut std::ffi::c_void),
            response: std::sync::mpsc::channel().0,
        },
    );
    if !sent {
        log::error!("Bridge: rebindSurface - compositor channel not initialized");
        ndk_sys::ANativeWindow_release(native_window);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_sendRelativeMotion(
    _env: JNIEnv,
    _class: JClass,
    dx: jfloat,
    dy: jfloat,
    time: jint,
) {
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::RelativeMotion {
            dx,
            dy,
            time: time as u32,
        },
    );
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_sendTrackpadClick(
    _env: JNIEnv,
    _class: JClass,
    state: jint,
    button: jint,
    time: jint,
) {
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::TrackpadClick {
            state,
            button,
            time: time as u32,
        },
    );
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_setInputLatencyTrace(
    _env: JNIEnv,
    _class: JClass,
    enabled: jboolean,
) {
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::SetInputLatencyTrace {
            enabled: enabled != 0,
        },
    );
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_sendTrackpadScroll(
    _env: JNIEnv,
    _class: JClass,
    dx: jfloat,
    dy: jfloat,
    time: jint,
    finished: jboolean,
) {
    crate::android::command_channel::send_command(
        crate::android::command_channel::JniCommand::TrackpadScroll {
            dx,
            dy,
            time: time as u32,
            finished: finished != 0,
        },
    );
}
