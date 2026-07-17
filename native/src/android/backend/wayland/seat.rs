use crate::android::backend::wayland::engine_timing;
#[cfg(feature = "smithay_android")]
use crate::android::backend::smithay_backend::RenderItem;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::output_management::OutputManagementState;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::shell::WindowElement;
#[cfg(feature = "smithay_android")]
use libc;
#[cfg(feature = "smithay_android")]
use smithay::backend::allocator::{Format, Fourcc, Modifier};
#[cfg(feature = "smithay_android")]
#[allow(unused_imports)]
use smithay::desktop::space::SpaceElement;
#[cfg(feature = "smithay_android")]
use smithay::desktop::{PopupManager, Space, Window};
#[cfg(feature = "smithay_android")]
use smithay::input::keyboard::{ModifiersState, XkbConfig};
#[cfg(feature = "smithay_android")]
use smithay::input::pointer::CursorImageStatus;
#[cfg(feature = "smithay_android")]
use smithay::input::pointer::CursorImageSurfaceData;
#[cfg(feature = "smithay_android")]
use smithay::input::Seat;
#[cfg(feature = "smithay_android")]
use smithay::input::SeatState;
#[cfg(feature = "smithay_android")]
use smithay::output::{Mode as OutputMode, Output, PhysicalProperties, Scale, Subpixel};
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::{Client, DisplayHandle, Resource};
#[cfg(feature = "smithay_android")]
use smithay::utils::Logical;
#[cfg(feature = "smithay_android")]
use smithay::utils::Point;
#[cfg(feature = "smithay_android")]
use smithay::utils::Transform;
#[cfg(feature = "smithay_android")]
use smithay::utils::{Serial, SERIAL_COUNTER};
#[cfg(feature = "smithay_android")]
use smithay::wayland::compositor::CompositorState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::cursor_shape::CursorShapeManagerState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::dmabuf::{DmabufFeedbackBuilder, DmabufGlobal, DmabufState};
#[cfg(feature = "smithay_android")]
use smithay::wayland::ext_workspace::{ExtWorkspaceHandler, ExtWorkspaceManagerState};
#[cfg(feature = "smithay_android")]
use smithay::wayland::foreign_toplevel_list::ForeignToplevelHandle;
#[cfg(feature = "smithay_android")]
use smithay::wayland::foreign_toplevel_list::ForeignToplevelListState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::fractional_scale::FractionalScaleManagerState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::idle_inhibit::IdleInhibitManagerState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::input_method::InputMethodManagerState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::output::OutputManagerState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::pointer_constraints::PointerConstraintsState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::pointer_gestures::PointerGesturesState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::presentation::PresentationState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::relative_pointer::RelativePointerManagerState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::seat::WaylandFocus;
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::data_device::DataDeviceState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::primary_selection::PrimarySelectionState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::selection::wlr_data_control::DataControlState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::shell::wlr_layer::WlrLayerShellState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::shell::xdg::decoration::XdgDecorationState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::shell::xdg::XdgShellState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::shm::ShmState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::single_pixel_buffer::SinglePixelBufferState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::text_input::{TextInputManagerState, TextInputSeat};
#[cfg(feature = "smithay_android")]
use smithay::wayland::viewporter::ViewporterState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::virtual_keyboard::VirtualKeyboardManagerState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::xdg_activation::XdgActivationState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::commit_timing::CommitTimingManagerState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::content_type::ContentTypeState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::session_lock::SessionLockManagerState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::shell::xdg::dialog::XdgDialogState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::xdg_toplevel_icon::XdgToplevelIconManager;
#[cfg(feature = "smithay_android")]
use smithay::wayland::image_capture_source::{ImageCaptureSourceState, OutputCaptureSourceState};
#[cfg(feature = "smithay_android")]
use smithay::wayland::image_copy_capture::ImageCopyCaptureState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::tearing_control::TearingControlState;
#[cfg(feature = "smithay_android")]
use smithay::wayland::xwayland_shell::XWaylandShellState;
#[cfg(feature = "smithay_android")]
use smithay::xwayland::X11Wm;
#[cfg(feature = "smithay_android")]
use smithay::xwayland::XWaylandClientData;
#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::server::WaylandClientState;
#[cfg(feature = "smithay_android")]
use std::collections::{HashMap, HashSet};
#[cfg(feature = "smithay_android")]
use std::sync::atomic::AtomicBool;
#[cfg(feature = "smithay_android")]
use std::sync::Arc;
#[cfg(feature = "smithay_android")]
use std::sync::Mutex;
#[cfg(feature = "smithay_android")]
use std::time::Instant;
#[cfg(feature = "smithay_android")]
use xkbcommon::xkb::{Context as XkbContext, Keymap as XkbKeymap};

// ── Helpers ──────────────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WinlandInputMode {
    Touch = 1,
    Trackpad = 2,
    Mouse = 4,
}

#[cfg(feature = "smithay_android")]
impl WinlandInputMode {
    pub(crate) fn from_bits(bits: i32) -> Self {
        match bits {
            1 => WinlandInputMode::Touch,
            2 => WinlandInputMode::Trackpad,
            4 => WinlandInputMode::Mouse,
            _ => {
                log::warn!(
                    "WinlandInputMode: invalid bits {}, defaulting to Touch",
                    bits
                );
                WinlandInputMode::Touch
            }
        }
    }

    pub(crate) fn to_bits(self) -> i32 {
        self as i32
    }
}

#[cfg(feature = "smithay_android")]
impl Default for WinlandInputMode {
    fn default() -> Self {
        WinlandInputMode::Touch
    }
}

#[cfg(feature = "smithay_android")]
fn init_stage<T>(name: &str, f: impl FnOnce() -> T) -> T {
    log::info!("SmithayRuntime: init stage={}", name);
    f()
}

#[cfg(feature = "smithay_android")]
fn compute_dpi_scale() -> f64 {
    // Return the cached wl_output.scale set by resolution preset chips.
    // logical_size always equals surface_size, so the scale is purely
    // a UI-density multiplier from the client's perspective.
    crate::android::command_channel::get_scale() as f64
}

// ── AndroidSeatRuntime ───────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
pub(crate) struct DebugKeymap(pub(crate) XkbKeymap);

#[cfg(feature = "smithay_android")]
impl std::fmt::Debug for DebugKeymap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XkbKeymap").finish_non_exhaustive()
    }
}

#[cfg(feature = "smithay_android")]
#[derive(Debug)]
#[allow(dead_code)]
pub struct AndroidSeatRuntime {
    pub(crate) compositor_state: CompositorState,
    pub(crate) shm_state: ShmState,
    pub(crate) dmabuf_state: DmabufState,
    pub(crate) dmabuf_global: DmabufGlobal,
    pub(crate) xdg_shell_state: XdgShellState,
    pub(crate) layer_shell_state: WlrLayerShellState,
    pub(crate) _xdg_decoration_state: XdgDecorationState,
    pub(crate) seat_state: SeatState<Self>,
    pub(crate) output_manager_state: OutputManagerState,
    pub(crate) data_device_state: DataDeviceState,
    pub(crate) display_handle: DisplayHandle,
    pub(crate) seat: Seat<Self>,
    pub(crate) output: Output,
    pub(crate) keyboard: Option<smithay::input::keyboard::KeyboardHandle<Self>>,
    pub(crate) pointer: smithay::input::pointer::PointerHandle<Self>,
    pub(crate) touch: smithay::input::touch::TouchHandle<Self>,
    pub(crate) current_input_mode: WinlandInputMode,
    pub(crate) primary_touch_id: Option<i32>,
    pub(crate) injected_events: u64,
    pub(crate) focused_surface: Option<WlSurface>,
    pub(crate) viewporter_state: ViewporterState,
    pub(crate) fractional_scale_state: FractionalScaleManagerState,
    pub(crate) presentation_state: PresentationState,
    pub(crate) cursor_shape_state: CursorShapeManagerState,
    pub(crate) _single_pixel_buffer_state: SinglePixelBufferState,
    pub(crate) xdg_activation_state: XdgActivationState,
    pub(crate) primary_selection_state: PrimarySelectionState,
    pub(crate) relative_pointer_state: RelativePointerManagerState,
    pub(crate) foreign_toplevel_list_state: ForeignToplevelListState,
    pub(crate) virtual_keyboard_state: VirtualKeyboardManagerState,
    pub(crate) data_control_state: DataControlState,
    pub(crate) space: Space<WindowElement>,
    pub(crate) popups: PopupManager,
    pub(crate) wl_to_window: HashMap<WlSurface, Window>,
    pub(crate) unmanaged_surfaces: Vec<WlSurface>,
    pub(crate) last_seat_dispatch: String,
    pub(crate) last_focus_decision: String,
    pub(crate) cursor_status: Option<CursorImageStatus>,
    pub(crate) last_cursor_mode: String,
    pub(crate) android_modifiers: ModifiersState,
    pub(crate) text_input_manager_state: TextInputManagerState,
    pub(crate) idle_inhibit_manager_state: IdleInhibitManagerState,
    pub(crate) idle_inhibit_count: u32,
    pub(crate) input_method_manager_state: InputMethodManagerState,
    pub(crate) reserved_top: i32,
    pub(crate) reserved_bottom: i32,
    pub(crate) swipe_starts: HashMap<i32, (f32, f32, f32, f32)>,
    pub(crate) active_touch_ids: HashSet<i32>,
    pub(crate) swipe_cycle_armed: bool,
    pub(crate) last_window_cycle_ms: u32,
    pub(crate) window_cycle_cooldown_ms: u32,
    pub(crate) rendering_active: bool,
    pub(crate) xwayland_shell_state: XWaylandShellState,
    pub(crate) x11_wm: Option<X11Wm>,
    pub(crate) xwayland_client: Option<Client>,
    pub(crate) popup_grab_active: bool,
    pub(crate) popup_grab_surface: Option<WlSurface>,
    pub(crate) relative_sensitivity: f32,
    pub(crate) mouse_last_pos: (f32, f32),
    pub(crate) pointer_button_pressed: bool,
    pub(crate) touch_two_finger_tap_active: bool,
    pub(crate) physical_size: (i32, i32),
    pub(crate) screen_size: (i32, i32),
    pub(crate) foreign_toplevel_handles: HashMap<WlSurface, ForeignToplevelHandle>,
    pub(crate) minimized: HashMap<WlSurface, Point<i32, Logical>>,
    pub(crate) maximize_restore: HashMap<WlSurface, Point<i32, Logical>>,
    pub(crate) fullscreen_restore:
        HashMap<WlSurface, smithay::utils::Rectangle<i32, smithay::utils::Logical>>,
    pub(crate) render_sender:
        crossbeam_channel::Sender<crate::android::backend::smithay_backend::RenderFrame>,
    pub(crate) presentation_receiver: crossbeam_channel::Receiver<u64>,
    pub(crate) buffer_last_frame: HashMap<WlBuffer, u64>,
    pub(crate) pending_buffer_releases: Vec<(WlBuffer, u64)>,
    pub(crate) latest_completed_frame: u64,
    pub(crate) input_latency_trace: bool,
    pub(crate) source_commit_sequence: u64,
    pub(crate) pointer_input_sequence: u64,
    pub(crate) pending_pointer_input: Option<(u64, Instant, u64)>,
    pub(crate) pointer_frame_timings: HashMap<u64, (u64, Instant)>,
    pub(crate) clipboard_text: Arc<Mutex<String>>,
    pub(crate) last_activation_serial: Option<Serial>,
    pub(crate) trackpad_anchor: Option<(f32, f32)>,
    pub(crate) trackpad_pointer_initialized: bool,
    pub(crate) trackpad_moved: bool,
    pub(crate) trackpad_tap_fingers: Vec<i32>,
    pub(crate) trackpad_hold_start_ms: u32,
    pub(crate) trackpad_dragging: bool,
    pub(crate) xkb_keymap: DebugKeymap,
    pub(crate) ext_workspace_state: ExtWorkspaceManagerState,
    pub(crate) pointer_gestures_state: PointerGesturesState,
    pub(crate) pointer_constraints_state: PointerConstraintsState,
    pub(crate) keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState,
    pub(crate) commit_timing_manager_state: CommitTimingManagerState,
    pub(crate) content_type_state: ContentTypeState,
    pub(crate) session_lock_state: SessionLockManagerState,
    pub(crate) xdg_dialog_state: XdgDialogState,
    pub(crate) xdg_toplevel_icon_manager: XdgToplevelIconManager,
    pub(crate) image_capture_source_state: ImageCaptureSourceState,
    pub(crate) output_capture_source_state: OutputCaptureSourceState,
    pub(crate) image_copy_capture_state: ImageCopyCaptureState,
    pub(crate) tearing_control_state: TearingControlState,
    pub(crate) output_management_state: OutputManagementState,
    pub(crate) x11_window_to_surface: HashMap<u32, WlSurface>,
    pub(crate) x11_pending_pings: HashMap<u32, u32>,
}

#[cfg(feature = "smithay_android")]
impl AndroidSeatRuntime {
    pub(crate) fn new(
        display: &DisplayHandle,
        width: i32,
        height: i32,
        render_sender: crossbeam_channel::Sender<crate::android::backend::smithay_backend::RenderFrame>,
        presentation_receiver: crossbeam_channel::Receiver<u64>,
    ) -> Result<Self, String> {
        log::info!("SmithayRuntime: init stage=compositor_state");
        let compositor_state =
            init_stage("compositor_state", || CompositorState::new_v6::<Self>(display));
        log::info!("SmithayRuntime: init stage=shm_state");
        let shm_state = init_stage("shm_state", || {
            let formats = vec![
                smithay::reexports::wayland_server::protocol::wl_shm::Format::Argb8888,
                smithay::reexports::wayland_server::protocol::wl_shm::Format::Xrgb8888,
                smithay::reexports::wayland_server::protocol::wl_shm::Format::Rgba8888,
                smithay::reexports::wayland_server::protocol::wl_shm::Format::Rgbx8888,
            ];
            log::info!(
                "WinlandV2: SHM formats being registered: {:?}",
                formats
                    .iter()
                    .map(|f| format!("{:?}", f))
                    .collect::<Vec<_>>()
            );
            ShmState::new::<Self>(display, formats)
        });
        log::info!("SmithayRuntime: init stage=dmabuf_state");
        let mut dmabuf_state = DmabufState::new();
        // Advertise only combinations proven importable by the KGSL Turnip
        // probe. Modifier::Invalid lets clients choose implementation-private
        // layouts (for example UBWC) that the current bridge cannot negotiate.
        let dmabuf_formats = vec![
            Format { code: Fourcc::Xbgr8888, modifier: Modifier::Linear },
            Format { code: Fourcc::Xrgb8888, modifier: Modifier::Linear },
            Format { code: Fourcc::Abgr8888, modifier: Modifier::Linear },
            Format { code: Fourcc::Argb8888, modifier: Modifier::Linear },
        ];
        // Aquamarine requires linux-dmabuf v4 feedback and resolves the tranche
        // device through libdrm. Use Android's real render-node identity as the
        // allocation hint while still advertising only KGSL-proven LINEAR formats.
        // The bridge never opens or modesets this node.
        // SELinux denies stat(2) on graphics_device from the app domain even
        // though the node identity is fixed and visible in Android's device
        // namespace. Encode renderD128's real 226:128 dev_t directly.
        let render_device = 0xe280 as nix::libc::dev_t;
        let dmabuf_feedback = DmabufFeedbackBuilder::new(render_device, dmabuf_formats)
            .build()
            .map_err(|error| format!("failed to build KGSL dmabuf feedback: {error:?}"))?;
        let dmabuf_global = dmabuf_state
            .create_global_with_default_feedback::<AndroidSeatRuntime>(display, &dmabuf_feedback);
        log::info!("SmithayRuntime: init stage=layer_shell_state");
        let layer_shell_state = init_stage("layer_shell_state", || {
            WlrLayerShellState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=xdg_shell_state");
        let xdg_shell_state = init_stage("xdg_shell_state", || XdgShellState::new::<Self>(display));
        log::info!("SmithayRuntime: init stage=xdg_decoration_state");
        let xdg_decoration_state = init_stage("xdg_decoration_state", || {
            XdgDecorationState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=output_manager_state");
        let output_manager_state = init_stage("output_manager_state", || {
            OutputManagerState::new_with_xdg_output::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=data_device_state");
        let data_device_state = init_stage("data_device_state", || {
            DataDeviceState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=viewporter_state");
        let viewporter_state =
            init_stage("viewporter_state", || ViewporterState::new::<Self>(display));
        log::info!("SmithayRuntime: init stage=fractional_scale_state");
        let fractional_scale_state = init_stage("fractional_scale_state", || {
            FractionalScaleManagerState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=presentation_state");
        let presentation_state = init_stage("presentation_state", || {
            PresentationState::new::<Self>(display, libc::CLOCK_MONOTONIC as u32)
        });
        log::info!("SmithayRuntime: init stage=cursor_shape_state");
        let cursor_shape_state = init_stage("cursor_shape_state", || {
            CursorShapeManagerState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=single_pixel_buffer_state");
        let _single_pixel_buffer_state = init_stage("single_pixel_buffer_state", || {
            SinglePixelBufferState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=xdg_activation_state");
        let xdg_activation_state = init_stage("xdg_activation_state", || {
            XdgActivationState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=primary_selection_state");
        let primary_selection_state = init_stage("primary_selection_state", || {
            PrimarySelectionState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=relative_pointer_state");
        let relative_pointer_state = init_stage("relative_pointer_state", || {
            RelativePointerManagerState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=foreign_toplevel_list_state");
        let foreign_toplevel_list_state = init_stage("foreign_toplevel_list_state", || {
            ForeignToplevelListState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=virtual_keyboard_state");
        let virtual_keyboard_state = init_stage("virtual_keyboard_state", || {
            VirtualKeyboardManagerState::new::<Self, _>(display, |_client| true)
        });
        log::info!("SmithayRuntime: init stage=data_control_state");
        let data_control_state = init_stage("data_control_state", || {
            DataControlState::new::<Self, _>(display, Some(&primary_selection_state), |_client| {
                true
            })
        });
        log::info!("SmithayRuntime: init stage=pointer_constraints_state");
        let pointer_constraints_state = init_stage("pointer_constraints_state", || {
            PointerConstraintsState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=text_input_manager_state");
        let text_input_manager_state = init_stage("text_input_manager_state", || {
            TextInputManagerState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=idle_inhibit_manager_state");
        let idle_inhibit_manager_state = init_stage("idle_inhibit_manager_state", || {
            IdleInhibitManagerState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=input_method_manager_state");
        let input_method_manager_state = init_stage("input_method_manager_state", || {
            InputMethodManagerState::new::<Self, _>(display, |_client| true)
        });

        log::info!("SmithayRuntime: init stage=ext_workspace_state");
        let ext_workspace_state = init_stage("ext_workspace_state", || {
            ExtWorkspaceManagerState::new::<Self>(display)
        });
        log::info!("SmithayRuntime: init stage=pointer_gestures_state");
        let pointer_gestures_state = init_stage("pointer_gestures_state", || {
            PointerGesturesState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=keyboard_shortcuts_inhibit_state");
        let keyboard_shortcuts_inhibit_state = init_stage("keyboard_shortcuts_inhibit_state", || {
            KeyboardShortcutsInhibitState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=commit_timing_manager_state");
        let commit_timing_manager_state = init_stage("commit_timing_manager_state", || {
            CommitTimingManagerState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=content_type_state");
        let content_type_state = init_stage("content_type_state", || {
            ContentTypeState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=session_lock_state");
        let session_lock_state = init_stage("session_lock_state", || {
            SessionLockManagerState::new::<Self, _>(display, |_client| true)
        });

        log::info!("SmithayRuntime: init stage=xdg_dialog_state");
        let xdg_dialog_state = init_stage("xdg_dialog_state", || {
            XdgDialogState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=xdg_toplevel_icon_manager");
        let mut xdg_toplevel_icon_manager = init_stage("xdg_toplevel_icon_manager", || {
            XdgToplevelIconManager::new::<Self>(display)
        });
        xdg_toplevel_icon_manager.add_icon_size(16);
        xdg_toplevel_icon_manager.add_icon_size(32);
        xdg_toplevel_icon_manager.add_icon_size(64);

        log::info!("SmithayRuntime: init stage=image_capture_source_state");
        let image_capture_source_state = ImageCaptureSourceState::new();

        log::info!("SmithayRuntime: init stage=output_capture_source_state");
        let output_capture_source_state = init_stage("output_capture_source_state", || {
            OutputCaptureSourceState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=image_copy_capture_state");
        let image_copy_capture_state = init_stage("image_copy_capture_state", || {
            ImageCopyCaptureState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=tearing_control_state");
        let tearing_control_state = init_stage("tearing_control_state", || {
            TearingControlState::new::<Self>(display)
        });

        log::info!("SmithayRuntime: init stage=output_management_state");
        let output_management_state =
            init_stage("output_management_state", || {
                OutputManagementState::create_global::<Self>(display)
            });

        log::info!(
            "SmithayRuntime: init stage=output size={}x{}",
            width,
            height
        );
        let phys = crate::android::command_channel::get_physical_size();
        log::info!(
            "SmithayRuntime: Creating output physical_size_mm=({}, {}) make=Winland model=Android",
            phys.0,
            phys.1
        );
        let output = Output::new(
            "android-0".to_string(),
            PhysicalProperties {
                size: (phys.0.max(1), phys.1.max(1)).into(),
                subpixel: Subpixel::Unknown,
                make: "Winland".into(),
                model: "Android".into(),
                serial_number: String::new(),
            },
        );
        let mode = OutputMode {
            size: (width, height).into(),
            refresh: 60000,
        };
        log::info!(
            "SmithayRuntime: Calling create_global for wl_output mode={}x{}",
            width,
            height
        );
        output.create_global::<Self>(display);
        let scale = compute_dpi_scale();
        output.change_current_state(
            Some(mode),
            Some(Transform::Normal),
            Some(Scale::Fractional(scale)),
            Some((0, 0).into()),
        );
        output.set_preferred(mode);
        {
            let pp = output.physical_properties();
            let loc = output.current_location();
            log::info!(
                "SmithayRuntime: Output inner state after creation: make={:?} model={:?} phys_size=({},{}) loc=({},{}) cur_mode={:?} modes={:?}",
                pp.make, pp.model,
                pp.size.w, pp.size.h,
                loc.x, loc.y,
                output.current_mode(),
                output.modes()
            );
        }
        log::info!(
            "SmithayRuntime: wl_output global created ({}x{}@60)",
            width,
            height
        );

        log::info!("SmithayRuntime: init stage=seat_state");
        let mut seat_state = init_stage("seat_state", SeatState::<Self>::new);
        log::info!("SmithayRuntime: creating wl_seat global");
        let mut seat = init_stage("seat_creation", || {
            seat_state.new_wl_seat(display, "android-seat")
        });
        log::info!("SmithayRuntime: init stage=keyboard");

        let context = XkbContext::new(0);

        if let Ok(root) = std::env::var("XKB_CONFIG_ROOT") {
            let rules_path = std::path::Path::new(&root).join("rules/evdev");
            if rules_path.exists() {
                log::info!(
                    "SmithayRuntime: Verified XKB rules exist at {}",
                    rules_path.display()
                );
            } else {
                log::error!(
                    "SmithayRuntime: XKB rules MISSING at {}",
                    rules_path.display()
                );
            }
        }

        let xkb_keymap = XkbKeymap::new_from_names(&context, "evdev", "pc105", "us,ara", "", None, 0)
            .ok_or_else(|| {
            "failed to load default xkb keymap (tried evdev/pc105/us,ara fallback)".to_string()
        })?;

        let keyboard = Some(
            seat.add_keyboard(XkbConfig { layout: "us,ara", ..Default::default() }, 200, 25)
                .expect("failed to add keyboard"),
        );
        log::info!("SmithayRuntime: xkb initialized successfully with evdev/pc105/us,ara");

        log::info!("SmithayRuntime: init stage=pointer");
        let pointer = init_stage("pointer", || seat.add_pointer());
        log::info!("SmithayRuntime: init stage=touch");
        let touch = init_stage("touch", || seat.add_touch());
        log::info!("SmithayRuntime: init stage=complete");

        let mut space = Space::default();
        space.map_output(&output, (0, 0));
        log::info!(
            "SmithayRuntime: Space mapped output '{}' at (0,0)",
            output.name()
        );

        Ok(Self {
            compositor_state,
            shm_state,
            dmabuf_state,
            dmabuf_global,
            xdg_shell_state,
            layer_shell_state,
            _xdg_decoration_state: xdg_decoration_state,
            seat_state,
            output_manager_state,
            data_device_state,
            display_handle: display.clone(),
            seat,
            output,
            keyboard,
            pointer,
            touch,
            current_input_mode: WinlandInputMode::Touch,
            primary_touch_id: None,
            injected_events: 0,
            focused_surface: None,
            space,
            popups: PopupManager::default(),
            wl_to_window: HashMap::new(),
            unmanaged_surfaces: Vec::new(),
            last_seat_dispatch: "none".to_string(),
            last_focus_decision: "none".to_string(),
            cursor_status: None,
            last_cursor_mode: "fallback:named:Default".to_string(),
            android_modifiers: ModifiersState::default(),
            text_input_manager_state,
            idle_inhibit_manager_state,
            idle_inhibit_count: 0,
            input_method_manager_state,
            reserved_top: 0,
            reserved_bottom: 0,
            swipe_starts: HashMap::new(),
            active_touch_ids: HashSet::new(),
            swipe_cycle_armed: false,
            last_window_cycle_ms: 0,
            window_cycle_cooldown_ms: 250,
            rendering_active: true,
            minimized: HashMap::new(),
            maximize_restore: HashMap::new(),
            fullscreen_restore: HashMap::new(),
            xwayland_shell_state: init_stage("xwayland_shell_state", || {
                XWaylandShellState::new::<Self>(display)
            }),
            x11_wm: None,
            xwayland_client: None,
            popup_grab_active: false,
            popup_grab_surface: None,
            relative_sensitivity: 1.0,
            mouse_last_pos: (0.0, 0.0),
            pointer_button_pressed: false,
            touch_two_finger_tap_active: false,
            physical_size: crate::android::command_channel::get_physical_size(),
            screen_size: (width, height),
            foreign_toplevel_handles: HashMap::new(),
            viewporter_state,
            fractional_scale_state,
            presentation_state,
            cursor_shape_state,
            _single_pixel_buffer_state,
            xdg_activation_state,
            primary_selection_state,
            relative_pointer_state,
            foreign_toplevel_list_state,
            virtual_keyboard_state,
            data_control_state,
            render_sender,
            presentation_receiver,
            buffer_last_frame: HashMap::new(),
            pending_buffer_releases: Vec::new(),
            latest_completed_frame: 0,
            input_latency_trace: false,
            source_commit_sequence: 0,
            pointer_input_sequence: 0,
            pending_pointer_input: None,
            pointer_frame_timings: HashMap::new(),
            clipboard_text: Arc::new(Mutex::new(String::new())),
            last_activation_serial: None,
            trackpad_anchor: None,
            trackpad_pointer_initialized: false,
            trackpad_moved: false,
            trackpad_tap_fingers: Vec::new(),
            trackpad_hold_start_ms: 0,
            trackpad_dragging: false,
            xkb_keymap: DebugKeymap(xkb_keymap),
            ext_workspace_state,
            pointer_gestures_state,
            pointer_constraints_state,
            keyboard_shortcuts_inhibit_state,
            commit_timing_manager_state,
            content_type_state,
            session_lock_state,
            xdg_dialog_state,
            xdg_toplevel_icon_manager,
            image_capture_source_state,
            output_capture_source_state,
            image_copy_capture_state,
            tearing_control_state,
            output_management_state,
            x11_window_to_surface: HashMap::new(),
            x11_pending_pings: HashMap::new(),
        })
    }

    /// إعادة حساب المساحة المحجوزة من طبقات wlr_layer الحية
    pub(crate) fn recalculate_reserved_zones(&mut self) {
        let mut top = 0;
        let mut bottom = 0;
        for layer in self.layer_shell_state.layer_surfaces() {
            let ez: i32 = layer.with_cached_state(|cached| cached.exclusive_zone.into());
            if ez > 0 {
                match layer.with_cached_state(|cached| cached.layer) {
                    smithay::wayland::shell::wlr_layer::Layer::Top => top = top.max(ez),
                    smithay::wayland::shell::wlr_layer::Layer::Bottom => bottom = bottom.max(ez),
                    _ => {}
                }
            }
        }
        self.reserved_top = top;
        self.reserved_bottom = bottom;
        log::debug!(
            "SmithayRuntime: recalculated reserved zones top={} bottom={}",
            top,
            bottom
        );
    }
}

#[cfg(feature = "smithay_android")]
impl ExtWorkspaceHandler for AndroidSeatRuntime {
    fn ext_workspace_state(&mut self) -> &mut ExtWorkspaceManagerState {
        &mut self.ext_workspace_state
    }
}

impl AndroidSeatRuntime {
    pub fn update_output_mode(&mut self, width: i32, height: i32, scale_override: Option<f64>) {
        // wl_output.mode always reports surface_size (the full framebuffer).
        // Use scale_override from wlr-output-management if provided,
        // otherwise fall back to the Android-side cached scale.
        self.physical_size = crate::android::command_channel::get_physical_size();
        self.screen_size = (width, height);
        let scale = scale_override.unwrap_or_else(compute_dpi_scale);
        log::info!(
            "SmithayRuntime: updating output mode to {}x{} scale={} (override={:?}) physical={:?}",
            width,
            height,
            scale,
            scale_override,
            self.physical_size
        );
        let mode = OutputMode {
            size: (width, height).into(),
            refresh: 60000,
        };
        self.output.change_current_state(
            Some(mode),
            Some(Transform::Normal),
            Some(Scale::Fractional(scale)),
            None,
        );
        // Propagate the new scale to XWayland's client_scale so X11Wm
        // converts X11 coordinates correctly (not locked at 1.0).
        if let Some(ref xc) = self.xwayland_client {
            let cs = xc.get_data::<WaylandClientState>()
                .map(|s| &s.compositor_state)
                .or_else(|| xc.get_data::<XWaylandClientData>()
                    .map(|d| &d.compositor_state));
            if let Some(c) = cs {
                c.set_client_scale(scale);
                log::info!("XWayland: client_scale set to {}", scale);
            }
        }
        self.output.set_preferred(mode);
        self.space.map_output(&self.output, (0, 0));
        let pp = self.output.physical_properties();
        log::info!(
            "SmithayRuntime: after output update make={:?} model={:?} phys=({},{}) cur_mode={:?}",
            pp.make,
            pp.model,
            pp.size.w,
            pp.size.h,
            self.output.current_mode()
        );
        self.render_all();
    }

    pub(crate) fn usable_screen_size(&self) -> (i32, i32) {
        let (log_w, log_h) = crate::android::command_channel::get_logical_size();
        if log_w > 0 && log_h > 0 {
            let usable_h = (log_h - self.reserved_top - self.reserved_bottom).max(200);
            (log_w.max(200), usable_h)
        } else {
            let (w, h) = crate::android::command_channel::get_surface_size();
            let (w, h) = if w > 0 && h > 0 { (w, h) } else { (1080, 1920) };
            let usable_h = (h - self.reserved_top - self.reserved_bottom).max(200);
            (w.max(200), usable_h)
        }
    }

    pub(crate) fn close_surface(&mut self, surface: WlSurface) {
        if let Some(window) = self.wl_to_window.get(&surface) {
            if let Some(x11) = window.x11_surface() {
                let _ = x11.close();
                log::info!("SmithayRuntime: closed X11 surface {:?}", surface.id());
                return;
            }
        }
        if let Some(client) = surface.client() {
            let err = smithay::reexports::wayland_server::backend::protocol::ProtocolError {
                code: 0,
                object_id: 0,
                object_interface: String::new(),
                message: String::new(),
            };
            client.kill(&self.display_handle, err);
            log::info!(
                "SmithayRuntime: killed client for surface {:?}",
                surface.id()
            );
        }
    }

    pub(crate) fn toggle_minimize(&mut self, surface: WlSurface) {
        if let Some(pos) = self.minimized.remove(&surface) {
            if let Some(window) = self.wl_to_window.get(&surface) {
                if let Some(x11) = window.x11_surface() {
                    let _ = x11.set_hidden(false);
                }
                self.space
                    .map_element(WindowElement(window.clone()), pos, false);
            }
            log::info!(
                "SmithayRuntime: restored minimized surface {:?}",
                surface.id()
            );
        } else {
            if let Some(window) = self.wl_to_window.get(&surface) {
                if let Some(x11) = window.x11_surface() {
                    let _ = x11.set_hidden(true);
                }
                if let Some(loc) = self.space.element_location(&WindowElement(window.clone())) {
                    self.minimized.insert(surface.clone(), loc);
                }
                self.space.unmap_elem(&WindowElement(window.clone()));
            }
            log::info!("SmithayRuntime: minimized surface {:?}", surface.id());
        }
        self.render_all();
    }

    pub(crate) fn toggle_maximize(&mut self, surface: WlSurface) {
        if let Some(pos) = self.maximize_restore.remove(&surface) {
            if let Some(window) = self.wl_to_window.get(&surface) {
                if let Some(x11) = window.x11_surface() {
                    let _ = x11.set_maximized(false);
                }
                self.space
                    .relocate_element(&WindowElement(window.clone()), pos);
            }
        } else {
            if let Some(window) = self.wl_to_window.get(&surface) {
                if let Some(loc) = self.space.element_location(&WindowElement(window.clone())) {
                    self.maximize_restore.insert(surface.clone(), loc);
                }
                if let Some(x11) = window.x11_surface() {
                    let _ = x11.set_maximized(true);
                    let (w, h) = self.usable_screen_size();
                    let rect = smithay::utils::Rectangle::new(
                        (0, self.reserved_top).into(),
                        (w, h).into(),
                    );
                    let _ = x11.configure(rect);
                }
                self.space
                    .relocate_element(&WindowElement(window.clone()), (0, self.reserved_top));
            }
        }
        self.render_all();
    }

    pub(crate) fn prune_dead_surfaces(&mut self) {
        self.space.refresh();
        self.popups.cleanup();

        // إزالة الأسطح الميتة من الجداول
        self.wl_to_window.retain(|s, _| s.is_alive());
        self.unmanaged_surfaces.retain(|s| s.is_alive());
        self.minimized.retain(|s, _| s.is_alive());
        self.maximize_restore.retain(|s, _| s.is_alive());
        self.fullscreen_restore.retain(|s, _| s.is_alive());
        self.foreign_toplevel_handles.retain(|s, handle| {
            if !s.is_alive() {
                self.foreign_toplevel_list_state.remove_toplevel(handle);
                false
            } else {
                true
            }
        });

        if self
            .popup_grab_surface
            .as_ref()
            .map_or(false, |s| !s.is_alive())
        {
            self.popup_grab_active = false;
            self.popup_grab_surface = None;
        }

        self.swipe_starts.retain(|_, v| {
            let s = v;
            s.0 >= 0.0 || s.1 >= 0.0
        });

        if let Some(ref fs) = self.focused_surface.clone() {
            if !fs.is_alive() {
                self.focused_surface = None;
                self.last_focus_decision = "focus_cleared:surface_dead".to_string();
            }
        }

        // إعادة حساب المساحة المحجوزة بعد إزالة الأسطح الميتة
        self.recalculate_reserved_zones();
    }

    pub(crate) fn sync_text_input_to_android(&self) {
        let mut has_active = false;
        self.seat.text_input().with_active_text_input(|_, _| {
            has_active = true;
        });
        notify_android_ime(has_active);
    }

    pub(crate) fn dismiss_popup(&mut self, popup_surface: &WlSurface) {
        if let Some(popup) = self.popups.find_popup(popup_surface) {
            if let Ok(root) = smithay::desktop::find_popup_root_surface(&popup) {
                let _ = PopupManager::dismiss_popup(&root, &popup);
            }
        }
        self.unmanaged_surfaces.retain(|s| s != popup_surface);
        if self.popup_grab_surface.as_ref() == Some(popup_surface) {
            self.popup_grab_active = false;
            self.popup_grab_surface = None;
        }
        if self.focused_surface.as_ref() == Some(popup_surface) {
            self.focused_surface = None;
            let candidate = self.choose_focus_candidate();
            let _ = self.apply_focus_candidate("popup_dismissed", candidate);
        }
        log::debug!(
            "SmithayRuntime: dismissed popup surface {:?}",
            popup_surface.id()
        );
    }

    pub(crate) fn set_focused_surface(&mut self, surface: &WlSurface) {
        self.focused_surface = Some(surface.clone());
    }

    pub(crate) fn set_input_mode(&mut self, mode: WinlandInputMode) {
        self.current_input_mode = mode;
        log::info!("SmithayRuntime: input mode -> {:?}", mode);
    }

    pub(crate) fn clear_focused_surface(&mut self) {
        self.focused_surface = None;
        if let Some(keyboard) = self.keyboard.clone() {
            keyboard.set_focus(self, None, SERIAL_COUNTER.next_serial());
        }
        log::info!("SmithayRuntime: focus cleared");
    }

    fn shm_slice<'a>(
        ptr: *const u8,
        len: usize,
        info: &smithay::wayland::shm::BufferData,
    ) -> Option<&'a [u8]> {
        let expected = info.offset as usize + info.stride as usize * info.height as usize;
        if len < expected {
            log::warn!(
                "SHM buffer corrupted: pool_len={} < expected={} (offset={} stride={} height={})",
                len,
                expected,
                info.offset,
                info.stride,
                info.height
            );
            return None;
        }
        Some(unsafe { std::slice::from_raw_parts(ptr, len) })
    }

    fn get_surface_buffer(wl_surface: &WlSurface) -> Option<WlBuffer> {
        use smithay::backend::renderer::utils::RendererSurfaceStateUserData;
        use smithay::wayland::compositor::{with_states, BufferAssignment, SurfaceAttributes};

        with_states(wl_surface, |states| {
            let mut attrs = states.cached_state.get::<SurfaceAttributes>();
            attrs
                .current()
                .buffer
                .as_ref()
                .and_then(|assignment| match assignment {
                    BufferAssignment::NewBuffer(buffer) => Some(buffer.clone()),
                    _ => None,
                })
                .or_else(|| {
                    states
                        .data_map
                        .get::<RendererSurfaceStateUserData>()
                        .and_then(|data| data.lock().ok())
                        .and_then(|guard| {
                            guard.buffer().map(|b| {
                                let wlb: &WlBuffer = b;
                                wlb.clone()
                            })
                        })
                })
        })
    }

    fn send_frame_callback(wl_surface: &WlSurface) {
        use smithay::wayland::compositor::{with_states, SurfaceAttributes};
        let now = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            / 1_000_000) as u32;

        with_states(wl_surface, |states| {
            let mut attrs = states.cached_state.get::<SurfaceAttributes>();
            for cb in attrs.current().frame_callbacks.drain(..) {
                cb.done(now);
            }
        });
    }

    fn try_read_dmabuf_via_mmap(fd: std::os::raw::c_int, offset: usize, stride: usize, width: usize, height: usize) -> Option<Vec<u8>> {
        let map_len = offset + stride * height;
        let ptr = unsafe {
            libc::mmap(std::ptr::null_mut(), map_len,
                       libc::PROT_READ, libc::MAP_SHARED, fd, 0)
        };
        if ptr == libc::MAP_FAILED {
            log::warn!("dmabuf mmap FAILED fd={} map_len={}", fd, map_len);
            return None;
        }
        let mut pixels = Vec::with_capacity(width * height * 4);
        unsafe {
            let base = (ptr as *const u8).add(offset);
            for row in 0..height {
                let slice = std::slice::from_raw_parts(
                    base.add(row * stride), width * 4);
                pixels.extend_from_slice(slice);
            }
            libc::munmap(ptr, map_len);
        }
        if pixels.is_empty() {
            return None;
        }
        Some(pixels)
    }

    fn try_read_dmabuf_via_pread(fd: std::os::raw::c_int, offset: usize, stride: usize, width: usize, height: usize) -> Option<Vec<u8>> {
        let read_len = offset + stride * height;
        let mut buf = vec![0u8; read_len];
        let nread = unsafe {
            libc::pread(fd, buf.as_mut_ptr() as *mut libc::c_void, read_len, 0)
        };
        if nread < 0 {
            log::warn!("dmabuf pread FAILED fd={} read_len={}", fd, read_len);
            return None;
        }
        let available = nread as usize;
        let mut pixels = Vec::with_capacity(width * height * 4);
        for row in 0..height {
            let row_off = offset + row * stride;
            if row_off + width * 4 <= available {
                pixels.extend_from_slice(&buf[row_off..row_off + width * 4]);
            }
        }
        if pixels.is_empty() {
            return None;
        }
        Some(pixels)
    }

    fn try_get_dmabuf_render_item(
        buffer: &WlBuffer,
        x: i32,
        y: i32,
        scale: f32,
        is_cursor: bool,
    ) -> Option<RenderItem> {
        use smithay::backend::allocator::Buffer;
        use smithay::wayland::dmabuf::get_dmabuf;
        use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

        let dmabuf = get_dmabuf(buffer).ok()?;
        let fd = dmabuf.handles().next()?;
        let fourcc = dmabuf.format().code as u32;
        let modifier: u64 = dmabuf.format().modifier.into();
        let width = dmabuf.width() as i32;
        let height = dmabuf.height() as i32;
        let stride = dmabuf.strides().next()?;
        let offset = dmabuf.offsets().next().unwrap_or(0);

        let duped = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD, 0) };
        if duped < 0 {
            log::warn!("dmabuf dup failed fd={}", fd.as_raw_fd());
            return None;
        }
        unsafe { libc::fcntl(duped, libc::F_SETFD, libc::FD_CLOEXEC); }
        let owned_fd = unsafe { OwnedFd::from_raw_fd(duped) };

        log::info!(
            "dmabuf render_item created fd={} fourcc=0x{:x} modifier=0x{:x} {}x{} stride={} offset={}",
            owned_fd.as_raw_fd(), fourcc, modifier, width, height, stride, offset
        );

        Some(RenderItem::DmaBuf {
            fd: owned_fd,
            fourcc,
            modifier,
            offset,
            stride,
            width,
            height,
            x,
            y,
            scale,
            is_cursor,
        })
    }

    fn track_surface_buffer_for_frame(&mut self, surface: &WlSurface, buffer: &WlBuffer, frame: u64) {
        let retired = smithay::wayland::compositor::with_states(surface, |states| {
            let mut attrs = states.cached_state.get::<smithay::wayland::compositor::SurfaceAttributes>();
            std::mem::take(&mut attrs.current().retired_buffers)
        });
        for old in retired {
            let last_frame = self.buffer_last_frame.remove(&old).unwrap_or(frame.saturating_sub(1));
            if last_frame <= self.latest_completed_frame {
                log::debug!("releasing retired wl_buffer immediately after compositor frame {}", last_frame);
                old.release();
            } else {
                self.pending_buffer_releases.push((old, last_frame));
            }
        }
        self.buffer_last_frame.insert(buffer.clone(), frame);
    }

    pub(crate) fn mark_pointer_input(&mut self) {
        if !self.input_latency_trace {
            return;
        }
        self.pointer_input_sequence = self.pointer_input_sequence.wrapping_add(1).max(1);
        self.pending_pointer_input = Some((
            self.pointer_input_sequence,
            Instant::now(),
            self.source_commit_sequence,
        ));
    }

    pub(crate) fn note_surface_commit(&mut self) {
        self.source_commit_sequence = self.source_commit_sequence.wrapping_add(1).max(1);
    }

    fn release_completed_buffers(&mut self) {
        let completed = self.latest_completed_frame;
        let mut waiting = Vec::new();
        for (buffer, frame) in self.pending_buffer_releases.drain(..) {
            if frame <= completed {
                log::debug!("releasing deferred wl_buffer after compositor frame {}", frame);
                buffer.release();
            } else {
                waiting.push((buffer, frame));
            }
        }
        self.pending_buffer_releases = waiting;
        self.buffer_last_frame.retain(|buffer, _| buffer.is_alive());
    }

    /// Render on the output clock, but complete pending frame callbacks only
    /// after the corresponding direct presentation or AHB release acknowledgement.
    pub(crate) fn frame_tick(&mut self) {
        let completed: Vec<u64> = self.presentation_receiver.try_iter().collect();
        if let Some(latest) = completed.iter().copied().max() {
            self.latest_completed_frame = self.latest_completed_frame.max(latest);
            self.release_completed_buffers();
        }
        for frame in &completed {
            if let Some((input, started)) = self.pointer_frame_timings.remove(frame) {
                log::info!(
                    "PADPUTER_INPUT_PRESENTED input={} frame={} latency_us={}",
                    input,
                    frame,
                    started.elapsed().as_micros()
                );
                if self.pending_pointer_input.is_some_and(|pending| pending.0 == input) {
                    self.pending_pointer_input = None;
                }
                self.pointer_frame_timings.retain(|_, timing| timing.0 != input);
            }
        }
        if !completed.is_empty() {
            log::debug!("presentation completed compositor frames {:?}; releasing frame callbacks", completed);
            for elem in self.space.elements() {
                if let Some(wl_surface) = elem.0.wl_surface() {
                    Self::send_frame_callback(wl_surface.as_ref());
                    for (popup, _) in PopupManager::popups_for_surface(wl_surface.as_ref()) {
                        Self::send_frame_callback(popup.wl_surface());
                    }
                }
            }
            for surface in &self.unmanaged_surfaces {
                Self::send_frame_callback(surface);
            }
        }
        self.render_all();
    }

    pub(crate) fn render_all(&mut self) {
        if !engine_timing::is_rendering_active() {
            return;
        }

        use smithay::wayland::compositor::{with_states, SurfaceAttributes};
        use smithay::wayland::shm::with_buffer_contents;

        let mut render_list: Vec<crate::android::backend::smithay_backend::RenderItem> = Vec::new();
        let mut tracked_buffers: Vec<(WlSurface, WlBuffer)> = Vec::new();
        let elem_count = self.space.elements().count();
        let unmanaged_count = self.unmanaged_surfaces.len();

        static RENDER_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let frame = RENDER_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let log_this = frame < 20 || frame % 600 == 0;

        if log_this {
            log::info!(
                "render_all #{}: {} space elements, {} unmanaged surfaces, {}",
                frame,
                elem_count,
                unmanaged_count,
                self.output
                    .current_mode()
                    .map(|m| format!("{}x{}", m.size.w, m.size.h))
                    .unwrap_or("none".into())
            );
        }

        for (idx, elem) in self.space.elements().enumerate() {
            if let Some(wl_surface) = elem.0.wl_surface() {
                let wl_surface = wl_surface.as_ref();

                let loc = self
                    .space
                    .element_location(elem)
                    .unwrap_or(Point::from((0, 0)));
                let buffer_info = Self::get_surface_buffer(wl_surface);

                let surface_scale = with_states(wl_surface, |states| {
                    let mut attrs = states.cached_state.get::<SurfaceAttributes>();
                    attrs.current().buffer_scale.max(1)
                }) as f32;

                if let Some(buffer) = buffer_info {
                    tracked_buffers.push((wl_surface.clone(), buffer.clone()));
                    let is_shm = buffer
                        .data::<smithay::wayland::shm::ShmBufferUserData>()
                        .is_some();
                    if !is_shm {
                        if let Some(item) = Self::try_get_dmabuf_render_item(&buffer, loc.x, loc.y, surface_scale, false) {
                            render_list.push(item);
                        } else if log_this {
                            log::warn!("  space[{}]: non-SHM and dmabuf read failed", idx);
                        }
                        continue;
                    }
                    match with_buffer_contents(&buffer, |ptr, len, info| {
                        let Some(slice) = Self::shm_slice(ptr, len, &info) else {
                            return;
                        };
                        let width = info.width as i32;
                        let height = info.height as i32;
                        let stride = info.stride as usize;
                        let offset = info.offset as usize;
                        let fmt = format!("{:?}", info.format);

                        let mut pixels = Vec::with_capacity((width * height * 4) as usize);
                        for y in 0..height {
                            let start = offset + (y as usize) * stride;
                            let end = start + (width as usize) * 4;
                            if end <= slice.len() {
                                pixels.extend_from_slice(&slice[start..end]);
                            } else {
                                if log_this {
                                    log::warn!("  space[{}]: row {} exceeds slice len {} (start={} end={})", idx, y, slice.len(), start, end);
                                }
                            }
                        }

                        if log_this {
                            let sample = if pixels.len() >= 8 {
                                format!(
                                    "p0=({},{},{},{}) p1=({},{},{},{})",
                                    pixels[0],
                                    pixels[1],
                                    pixels[2],
                                    pixels[3],
                                    pixels[4],
                                    pixels[5],
                                    pixels[6],
                                    pixels[7]
                                )
                            } else if pixels.len() >= 4 {
                                format!(
                                    "p0=({},{},{},{})",
                                    pixels[0], pixels[1], pixels[2], pixels[3]
                                )
                            } else {
                                "empty".to_string()
                            };
                            log::info!(
                                "  space[{}]: buf={}x{} stride={} offset={} format={} pixels={} loc={},{} slice_len={} sample={}",
                                idx, width, height, stride, offset, fmt, pixels.len(), loc.x, loc.y, slice.len(), sample,
                            );
                        }

                        if !pixels.is_empty() {
                            render_list.push(RenderItem::Shm { pixels, x: loc.x, y: loc.y, width, height, scale: surface_scale, is_cursor: false });
                        }
                    }) {
                        Ok(_) => {}
                        Err(e) => {
                            if log_this {
                                log::warn!(
                                    "  space[{}]: with_buffer_contents failed: {:?}",
                                    idx,
                                    e
                                );
                            }
                        }
                    }
                } else if log_this {
                    log::info!("  space[{}]: no buffer yet", idx);
                }

                for (popup, popup_loc) in PopupManager::popups_for_surface(wl_surface) {
                    let popup_surface = popup.wl_surface();
                    let abs_loc = loc + popup_loc;
                    let buffer_info = Self::get_surface_buffer(popup_surface);

                    let popup_scale = with_states(popup_surface, |states| {
                        let mut attrs = states.cached_state.get::<SurfaceAttributes>();
                        attrs.current().buffer_scale.max(1)
                    }) as f32;

                    if let Some(buffer) = buffer_info {
                        tracked_buffers.push((popup_surface.clone(), buffer.clone()));
                        if buffer
                            .data::<smithay::wayland::shm::ShmBufferUserData>()
                            .is_none()
                        {
                            if let Some(item) = Self::try_get_dmabuf_render_item(&buffer, abs_loc.x, abs_loc.y, popup_scale, false) {
                                render_list.push(item);
                            }
                            continue;
                        }
                        let _ = with_buffer_contents(&buffer, |ptr, len, info| {
                            let Some(slice) = Self::shm_slice(ptr, len, &info) else {
                                return;
                            };
                            let width = info.width as i32;
                            let height = info.height as i32;
                            let stride = info.stride as usize;
                            let offset = info.offset as usize;

                            let mut pixels = Vec::with_capacity((width * height * 4) as usize);
                            for y in 0..height {
                                let start = offset + (y as usize) * stride;
                                let end = start + (width as usize) * 4;
                                if end <= slice.len() {
                                    pixels.extend_from_slice(&slice[start..end]);
                                }
                            }

                            if !pixels.is_empty() {
                                render_list.push(RenderItem::Shm { pixels, x: abs_loc.x, y: abs_loc.y, width, height, scale: popup_scale, is_cursor: false });
                            }
                        });
                    }
                }
            }
        }

        for (idx, s) in self.unmanaged_surfaces.iter().enumerate() {
            // A committed wl_surface is not displayable until a protocol gives
            // it a role. Aquamarine uploads its cursor before the first pointer
            // enter, so it is briefly roleless before set_cursor assigns
            // CURSOR_IMAGE_ROLE. Never leak either state into desktop frames.
            let role = smithay::wayland::compositor::get_role(s);
            let is_cursor = role == Some(smithay::wayland::seat::CURSOR_IMAGE_ROLE)
                || matches!(
                    self.cursor_status.as_ref(),
                    Some(CursorImageStatus::Surface(cursor_surface)) if cursor_surface == s
                )
                || with_states(s, |states| {
                    states.data_map.get::<CursorImageSurfaceData>().is_some()
                });
            if role.is_none() || is_cursor {
                if log_this {
                    log::info!(
                        "  unmanaged[{}]: skip role={:?} cursor={} (not a desktop item)",
                        idx,
                        role,
                        is_cursor
                    );
                }
                continue;
            }

            let buffer_info = Self::get_surface_buffer(s);

            let surface_scale = with_states(s, |states| {
                let mut attrs = states.cached_state.get::<SurfaceAttributes>();
                attrs.current().buffer_scale.max(1)
            }) as f32;

            if let Some(buffer) = buffer_info {
                tracked_buffers.push((s.clone(), buffer.clone()));
                let is_shm = buffer
                    .data::<smithay::wayland::shm::ShmBufferUserData>()
                    .is_some();
                if !is_shm {
                    if let Some(item) = Self::try_get_dmabuf_render_item(&buffer, 0, self.reserved_top, surface_scale, false) {
                        render_list.push(item);
                    } else if log_this {
                        log::warn!("  unmanaged[{}]: non-SHM and dmabuf read failed", idx);
                    }
                    continue;
                }
                match with_buffer_contents(&buffer, |ptr, len, info| {
                    let Some(slice) = Self::shm_slice(ptr, len, &info) else {
                        return;
                    };
                    let width = info.width as i32;
                    let height = info.height as i32;
                    let stride = info.stride as usize;
                    let offset = info.offset as usize;
                    let fmt = format!("{:?}", info.format);

                    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
                    for y in 0..height {
                        let start = offset + (y as usize) * stride;
                        let end = start + (width as usize) * 4;
                        if end <= slice.len() {
                            pixels.extend_from_slice(&slice[start..end]);
                        } else {
                            if log_this {
                                log::warn!("  unmanaged[{}]: row {} exceeds slice len {} (start={} end={})", idx, y, slice.len(), start, end);
                            }
                        }
                    }

                    if log_this {
                        let sample = if pixels.len() >= 8 {
                            format!(
                                "p0=({},{},{},{}) p1=({},{},{},{})",
                                pixels[0],
                                pixels[1],
                                pixels[2],
                                pixels[3],
                                pixels[4],
                                pixels[5],
                                pixels[6],
                                pixels[7]
                            )
                        } else if pixels.len() >= 4 {
                            format!(
                                "p0=({},{},{},{})",
                                pixels[0], pixels[1], pixels[2], pixels[3]
                            )
                        } else {
                            "empty".to_string()
                        };
                        log::info!(
                            "  unmanaged[{}]: buf={}x{} stride={} offset={} format={} pixels={} loc={},{} slice_len={} sample={}",
                            idx, width, height, stride, offset, fmt, pixels.len(), 0, self.reserved_top, slice.len(), sample,
                        );
                    }

                    if !pixels.is_empty() {
                        render_list.push(RenderItem::Shm { pixels, x: 0, y: self.reserved_top, width, height, scale: surface_scale, is_cursor: false });
                    }
                }) {
                    Ok(_) => {}
                    Err(e) => {
                        if log_this {
                            log::warn!(
                                "  unmanaged[{}]: with_buffer_contents failed: {:?}",
                                idx,
                                e
                            );
                        }
                    }
                }
            } else if log_this {
                log::info!("  unmanaged[{}]: no buffer yet", idx);
            }
        }

        // ── Cursor overlay ──
        let cursor_pos = self.pointer.current_location();

        if let Some(ref cursor_status) = self.cursor_status {
            match cursor_status {
                CursorImageStatus::Hidden => {}
                CursorImageStatus::Named(_) => {
                    let (_cursor_pixels, cw, ch) = fallback_cursor_pixels();
                    let cx = cursor_pos.x as i32 - 1;
                    let cy = cursor_pos.y as i32 - 1;
                    if log_this {
                        log::info!(
                            "  cursor: named fallback at {},{} size {}x{}",
                            cx,
                            cy,
                            cw,
                            ch
                        );
                    }
                    if log_this {
                        log::info!("PADPUTER_OUTER_CURSOR_SOURCE kind=named width={} height={} hotspot=1,1", cw, ch);
                    }
                }
                CursorImageStatus::Surface(wl_surface) => {
                    if !wl_surface.is_alive() {
                        if log_this {
                            log::warn!("  cursor: surface dead, skipping");
                        }
                    } else {
                        let surface_scale = with_states(wl_surface, |states| {
                            let mut attrs = states.cached_state.get::<SurfaceAttributes>();
                            attrs.current().buffer_scale.max(1)
                        }) as f32;

                        let hotspot = with_states(wl_surface, |states| {
                            states
                                .data_map
                                .get::<CursorImageSurfaceData>()
                                .and_then(|d| d.lock().ok())
                                .map(|guard| guard.hotspot)
                                .unwrap_or(Point::from((0, 0)))
                        });

                        let cx = cursor_pos.x as i32 - hotspot.x;
                        let cy = cursor_pos.y as i32 - hotspot.y;

                        let buffer_info = Self::get_surface_buffer(wl_surface);
                        if let Some(buffer) = buffer_info {
                            tracked_buffers.push((wl_surface.clone(), buffer.clone()));
                            if buffer
                                .data::<smithay::wayland::shm::ShmBufferUserData>()
                                .is_none()
                            {
                                if let Some(item) = Self::try_get_dmabuf_render_item(&buffer, cx, cy, surface_scale, true)
                                {
                                    let _ = item;
                                    if log_this {
                                        log::info!("PADPUTER_OUTER_CURSOR_SOURCE kind=dmabuf hotspot={},{} scale={}", hotspot.x, hotspot.y, surface_scale);
                                    }
                                } else if log_this {
                                    log::warn!("  cursor: non-SHM and dmabuf read failed");
                                }
                            } else {
                                let _ = with_buffer_contents(&buffer, |ptr, len, info| {
                                    let Some(slice) = Self::shm_slice(ptr, len, &info) else {
                                        return;
                                    };
                                    let width = info.width as i32;
                                    let height = info.height as i32;
                                    let stride = info.stride as usize;
                                    let offset = info.offset as usize;

                                    let mut pixels =
                                        Vec::with_capacity((width * height * 4) as usize);
                                    for y in 0..height {
                                        let start = offset + (y as usize) * stride;
                                        let end = start + (width as usize) * 4;
                                        if end <= slice.len() {
                                            pixels.extend_from_slice(&slice[start..end]);
                                        }
                                    }

                                    if !pixels.is_empty() {
                                        if log_this {
                                            log::info!(
                                                "  cursor: surface {}x{} at {},{} scale={}",
                                                width,
                                                height,
                                                cx,
                                                cy,
                                                surface_scale
                                            );
                                        }
                                        if log_this {
                                            log::info!("PADPUTER_OUTER_CURSOR_SOURCE kind=shm width={} height={} stride={} hotspot={},{} bytes={} scale={}", width, height, stride, hotspot.x, hotspot.y, pixels.len(), surface_scale);
                                        }
                                    }
                                });
                            }
                        } else if log_this {
                            log::info!("  cursor: no buffer yet");
                        }
                    }
                }
            }
        }

        for (surface, buffer) in tracked_buffers {
            self.track_surface_buffer_for_frame(&surface, &buffer, frame);
        }

        if !render_list.is_empty() {
            if let Some((input, started, commit_at_input)) = self.pending_pointer_input {
                if self.source_commit_sequence > commit_at_input {
                    let first_association = !self
                        .pointer_frame_timings
                        .values()
                        .any(|timing| timing.0 == input);
                    if first_association {
                        log::info!(
                            "PADPUTER_INPUT_FRAME_ASSOCIATED input={} frame={} commit={} queue_us={}",
                            input,
                            frame,
                            self.source_commit_sequence,
                            started.elapsed().as_micros()
                        );
                    }
                    // Keep associating subsequent copies of this committed
                    // source buffer until one exact frame id is acknowledged;
                    // the AHB broker intentionally coalesces queued frames.
                    self.pointer_frame_timings.insert(frame, (input, started));
                    if self.pointer_frame_timings.len() > 64 {
                        if let Some(oldest) = self.pointer_frame_timings.keys().copied().min() {
                            self.pointer_frame_timings.remove(&oldest);
                        }
                    }
                }
            }
            let _ = self.render_sender.send(crate::android::backend::smithay_backend::RenderFrame {
                id: frame,
                items: render_list,
            });
        }

    }
}

// ── Fallback cursor bitmap ──────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
fn fallback_cursor_pixels() -> (Vec<u8>, i32, i32) {
    // 16x16 classic arrow cursor: tip at (0,0), white fill with black outline
    const W: i32 = 16;
    const H: i32 = 16;
    let mut pixels = vec![0u8; (W * H * 4) as usize];
    for y in 0..H {
        // expanding arrow head (upper triangle: y <= x)
        // then tapered back edge
        let rightmost = if y < 12 {
            y
        } else if y < 14 {
            12 - (y - 12) * 2
        } else if y < 16 {
            8 - (y - 14) * 2
        } else {
            0
        };
        for x in 0..=rightmost.min(W - 1) {
            // outline: outer edge pixels are black, inner are white
            let is_outline = x == 0 || x == rightmost || y == x || (y >= 12 && x == rightmost);
            let i = ((y * W + x) * 4) as usize;
            if is_outline || rightmost == 0 || x == y {
                pixels[i] = 0;
                pixels[i + 1] = 0;
                pixels[i + 2] = 0;
            } else {
                pixels[i] = 255;
                pixels[i + 1] = 255;
                pixels[i + 2] = 255;
            }
            pixels[i + 3] = 255;
        }
    }
    (pixels, W, H)
}

// ── IME notification ─────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
fn notify_android_ime(show: bool) {
    use std::sync::atomic::Ordering;
    static LAST_IME_STATE: AtomicBool = AtomicBool::new(false);
    if LAST_IME_STATE.swap(show, Ordering::Relaxed) == show {
        return;
    }
    crate::android::bridge_clipboard::set_ime_visible(show);
}
