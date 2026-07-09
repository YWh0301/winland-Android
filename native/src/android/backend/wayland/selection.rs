#[cfg(feature = "smithay_android")]
use std::io::Write;
#[cfg(feature = "smithay_android")]
use smithay::input::Seat;
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::{SelectionHandler, SelectionSource, SelectionTarget};
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::seat::AndroidSeatRuntime;

#[cfg(feature = "smithay_android")]
impl SelectionHandler for AndroidSeatRuntime {
    type SelectionUserData = String;

    fn new_selection(
        &mut self,
        ty: SelectionTarget,
        source: Option<SelectionSource>,
        _seat: Seat<Self>,
    ) {
        log::info!("[DD_DC] new_selection called: ty={:?}, has_source={}", ty, source.is_some());
        if ty != SelectionTarget::Clipboard && ty != SelectionTarget::Primary {
            return;
        }

        let Some(source) = source else {
            log::info!("[DD_DC] new_selection: source is None — clipboard cleared");
            return;
        };

        let text_mime = "text/plain".to_string();
        let utf8_mime = "text/plain;charset=utf-8".to_string();
        let mime = if source.mime_types().contains(&utf8_mime) {
            utf8_mime
        } else if source.mime_types().contains(&text_mime) {
            text_mime
        } else {
            return;
        };

        let (read_fd, write_fd) = match nix::unistd::pipe2(nix::fcntl::OFlag::O_CLOEXEC) {
            Ok(fds) => fds,
            Err(e) => {
                log::warn!("Clipboard: failed to create pipe: {}", e);
                return;
            }
        };

        // Read from the new source directly, not from the current clipboard.
        // new_selection is called BEFORE set_clipboard_selection updates the
        // seat state, so the current clipboard still points to the OLD source.
        // Using source.send() avoids the false Err(ServerSideSelection/NoSelection)
        // that request_data_device_client_selection would return when it reads
        // the not-yet-updated seat clipboard.
        source.send(mime, write_fd);

        let clipboard = self.clipboard_text.clone();
        std::thread::spawn(move || {
            use std::io::Read;
            use std::os::fd::{FromRawFd, IntoRawFd};
            let mut file = unsafe { std::fs::File::from_raw_fd(read_fd.into_raw_fd()) };
            let mut buf = String::new();
            if file.read_to_string(&mut buf).is_ok() {
                let text = buf.trim_end_matches('\0').to_string();
                if !text.is_empty() {
                    log::debug!("Clipboard: Wayland client set selection len={}", text.len());
                    if let Ok(mut guard) = clipboard.lock() {
                        *guard = text.clone();
                    }
                    crate::android::bridge_clipboard::set_clipboard_text(&text);
                    // Re-set clipboard as Compositor source so it survives client exit.
                    let text_for_update = text.clone();
                    crate::android::command_channel::send_command(
                        crate::android::command_channel::JniCommand::UpdateClipboard { text: text_for_update },
                    );
                    crate::android::command_channel::send_command(
                        crate::android::command_channel::JniCommand::WaylandClipboardToAndroid { text },
                    );
                }
            }
        });
    }

    fn send_selection(
        &mut self,
        ty: SelectionTarget,
        mime_type: String,
        fd: std::os::unix::io::OwnedFd,
        _seat: Seat<Self>,
        user_data: &Self::SelectionUserData,
    ) {
        log::info!("[DD_DC] send_selection called: ty={:?}, mime_type={}, user_data_len={}", ty, mime_type, user_data.len());
use std::os::fd::{FromRawFd, IntoRawFd};

        if ty != SelectionTarget::Clipboard && ty != SelectionTarget::Primary {
            return;
        }

        if mime_type.starts_with("text/plain") || mime_type == "UTF8_STRING" || mime_type == "STRING" || mime_type == "TEXT" {
            let text = user_data.clone();
            let raw_fd = fd.into_raw_fd();
            std::thread::spawn(move || {
                let mut file = unsafe { std::fs::File::from_raw_fd(raw_fd) };
                let _ = file.write_all(text.as_bytes());
            });
        }
    }
}
