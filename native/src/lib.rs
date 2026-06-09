use jni::JNIEnv;
use jni::objects::JClass;
use jni::sys::jint;
use std::sync::OnceLock;

/// Global JVM reference so native threads (compositor) can call back into Java.
static JAVA_VM: OnceLock<jni::JavaVM> = OnceLock::new();

/// Retrieve the stored JavaVM.  Returns `None` before the library is loaded.
pub(crate) fn java_vm() -> Option<&'static jni::JavaVM> {
    JAVA_VM.get()
}

/// JNI_OnLoad: called once when the native library is loaded by the JVM.
/// Stores the JavaVM for later use on non-JNI threads.
#[no_mangle]
pub extern "system" fn JNI_OnLoad(vm: jni::JavaVM, _reserved: *mut std::ffi::c_void) -> jint {
    let _ = JAVA_VM.set(vm);
    jni::sys::JNI_VERSION_1_6
}

mod compositor;
mod audio;
mod usb;
mod root;
mod distro;
pub mod android;
pub mod core;

use distro::DistroManager;

uniffi::setup_scaffolding!("winland_core");

#[uniffi::export(callback_interface)]
pub trait LogCallback: Send + Sync {
    fn on_log(&self, level: String, message: String);
}

#[derive(uniffi::Object)]
pub struct WinlandServer {
    base_path: String,
}

#[uniffi::export]
impl WinlandServer {
    #[uniffi::constructor]
    pub fn new(base_path: String) -> Self {
        Self { base_path }
    }

    pub fn install_distro(&self, name: String, _url: String, callback: Box<dyn LogCallback>) {
        let manager = DistroManager::new(&self.base_path);
        callback.on_log("INFO".to_string(), format!("Preparing chroot for {}", name));
        match manager.prepare_chroot(&name) {
            Ok(_) => {
                callback.on_log("SUCCESS".to_string(), format!("Chroot for {} prepared", name));
            }
            Err(e) => callback.on_log("ERROR".to_string(), format!("Error: {}", e)),
        }
    }

    pub fn list_distros(&self) -> Vec<String> {
        let distro_dir = std::path::Path::new(&self.base_path).join("distros");
        std::fs::read_dir(distro_dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok().map(|en| en.file_name().to_string_lossy().into_owned()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[uniffi::export]
pub fn get_version() -> String {
    "Winland Server v1.1.0 (ARM64 Production-Ready)".to_string()
}

#[uniffi::export]
pub fn init_compositor(distro_id: String) {
    if let Err(error) = compositor::spawn(&distro_id) {
        log::error!("init_compositor failed: {}", error);
    }
}

#[uniffi::export]
pub fn init_audio() {
    audio::init();
}

#[uniffi::export]
pub fn init_usb(fd: i32) {
    usb::init(fd);
}

#[uniffi::export]
pub fn get_root_mode() -> bool {
    root::is_root()
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_initAudioBridge(
    _env: JNIEnv,
    _class: JClass,
) {
    audio::init();
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_initUsbRedirection(
    _env: JNIEnv,
    _class: JClass,
    fd: jint,
) {
    usb::init(fd);
}

#[no_mangle]
pub extern "system" fn Java_com_winland_server_NativeBridge_closeUsbRedirection(
    _env: JNIEnv,
    _class: JClass,
) {
    usb::shutdown();
}

