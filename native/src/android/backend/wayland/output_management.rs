#[cfg(feature = "smithay_android")]
use crossbeam_channel::{Receiver, Sender};

#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_protocols_wlr::output_management::v1::server::{
    zwlr_output_configuration_head_v1::{self, ZwlrOutputConfigurationHeadV1},
    zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1},
    zwlr_output_head_v1::{self, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
    zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
};
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::backend::GlobalId;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::protocol::wl_output::Transform as WlTransform;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::WEnum;
#[cfg(feature = "smithay_android")]
use smithay::reexports::wayland_server::{Client, DataInit, DisplayHandle, New};

#[cfg(feature = "smithay_android")]
use crate::android::backend::wayland::seat::AndroidSeatRuntime;

// ── State ────────────────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
#[derive(Debug)]
pub(crate) struct OutputManagementState {
    #[allow(dead_code)]
    global: Option<GlobalId>,
}

#[cfg(feature = "smithay_android")]
impl OutputManagementState {
    pub fn new() -> Self {
        Self { global: None }
    }

    pub fn create_global<D>(display: &DisplayHandle) -> Self
    where
        D: smithay::reexports::wayland_server::GlobalDispatch<
                ZwlrOutputManagerV1,
                smithay::wayland::GlobalData,
            > + 'static,
    {
        let global =
            display.create_global::<D, ZwlrOutputManagerV1, _>(4, smithay::wayland::GlobalData);
        Self {
            global: Some(global),
        }
    }
}

// ── Channel messages ─────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
enum HeadAction {
    SetCustomMode(i32, i32, i32),
    SetPosition(i32, i32),
    SetTransform(i32),
    SetScale(f64),
    SetAdaptiveSync(bool),
}

// ── User data types ──────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
struct MgrUserData;

#[cfg(feature = "smithay_android")]
struct HeadUserData;

#[cfg(feature = "smithay_android")]
struct ModeUserData;

#[cfg(feature = "smithay_android")]
struct ConfigUserData {
    head_tx: Sender<HeadAction>,
    head_rx: Receiver<HeadAction>,
}

#[cfg(feature = "smithay_android")]
struct ConfigHeadUserData {
    tx: Sender<HeadAction>,
}

// ── Manager global binding ───────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl smithay::wayland::GlobalDispatch2<ZwlrOutputManagerV1, AndroidSeatRuntime>
    for smithay::wayland::GlobalData
{
    fn bind(
        &self,
        state: &mut AndroidSeatRuntime,
        handle: &DisplayHandle,
        client: &Client,
        resource: New<ZwlrOutputManagerV1>,
        data_init: &mut DataInit<'_, AndroidSeatRuntime>,
    ) {
        log::info!("SmithayRuntime: wlr-output-management manager bound");

        let head = match client
            .create_resource::<ZwlrOutputHeadV1, HeadUserData, AndroidSeatRuntime>(
                handle,
                4,
                HeadUserData,
            ) {
            Ok(h) => h,
            Err(e) => {
                log::error!("SmithayRuntime: failed to create head resource: {:?}", e);
                return;
            }
        };

        let mode = match client
            .create_resource::<ZwlrOutputModeV1, ModeUserData, AndroidSeatRuntime>(
                handle,
                3,
                ModeUserData,
            ) {
            Ok(m) => m,
            Err(e) => {
                log::error!("SmithayRuntime: failed to create mode resource: {:?}", e);
                return;
            }
        };

        let output = &state.output;
        let current_mode = output.current_mode().unwrap_or(smithay::output::Mode {
            size: (1920, 1080).into(),
            refresh: 60000,
        });
        let pp = output.physical_properties();
        let location = output.current_location();
        let scale_val = output.current_scale().fractional_scale();
        let transform = output.current_transform();

        let manager = data_init.init(resource, MgrUserData);

        manager.head(&head);
        head.mode(&mode);

        mode.size(current_mode.size.w, current_mode.size.h);
        mode.refresh(current_mode.refresh);
        mode.preferred();

        head.name(output.name().to_string());
        head.description(format!("{} {}", pp.make, pp.model));
        {
            let w = pp.size.w;
            let h = pp.size.h;
            if w > 0 && h > 0 {
                head.physical_size(w, h);
            }
        }
        head.enabled(1);
        head.current_mode(&mode);
        head.position(location.x, location.y);
        head.transform(transform.into());
        head.scale(scale_val);
        head.make(pp.make.clone());
        head.model(pp.model.clone());
        head.serial_number(pp.serial_number.clone());

        manager.done(1);

        log::info!(
            "SmithayRuntime: wlr-output-management head '{}' {}x{} scale={:.1}",
            output.name(),
            current_mode.size.w,
            current_mode.size.h,
            scale_val,
        );
    }
}

// ── Manager dispatch ─────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl smithay::wayland::Dispatch2<ZwlrOutputManagerV1, AndroidSeatRuntime> for MgrUserData {
    fn request(
        &self,
        _state: &mut AndroidSeatRuntime,
        _client: &Client,
        _resource: &ZwlrOutputManagerV1,
        request: zwlr_output_manager_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, AndroidSeatRuntime>,
    ) {
        match request {
            zwlr_output_manager_v1::Request::CreateConfiguration { id, serial } => {
                log::info!(
                    "SmithayRuntime: wlr-output-management create_configuration serial={}",
                    serial
                );
                let (head_tx, head_rx) = crossbeam_channel::unbounded();
                data_init.init(id, ConfigUserData { head_tx, head_rx });
            }
            zwlr_output_manager_v1::Request::Stop => {
                log::info!("SmithayRuntime: wlr-output-management stop");
            }
            _ => {}
        }
    }
}

// ── Head dispatch ────────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl smithay::wayland::Dispatch2<ZwlrOutputHeadV1, AndroidSeatRuntime> for HeadUserData {
    fn request(
        &self,
        _state: &mut AndroidSeatRuntime,
        _client: &Client,
        _resource: &ZwlrOutputHeadV1,
        request: zwlr_output_head_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, AndroidSeatRuntime>,
    ) {
        match request {
            zwlr_output_head_v1::Request::Release => {}
            _ => {}
        }
    }
}

// ── Mode dispatch ────────────────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl smithay::wayland::Dispatch2<ZwlrOutputModeV1, AndroidSeatRuntime> for ModeUserData {
    fn request(
        &self,
        _state: &mut AndroidSeatRuntime,
        _client: &Client,
        _resource: &ZwlrOutputModeV1,
        request: zwlr_output_mode_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, AndroidSeatRuntime>,
    ) {
        match request {
            zwlr_output_mode_v1::Request::Release => {}
            _ => {}
        }
    }
}

// ── Configuration dispatch ───────────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl smithay::wayland::Dispatch2<ZwlrOutputConfigurationV1, AndroidSeatRuntime> for ConfigUserData {
    fn request(
        &self,
        state: &mut AndroidSeatRuntime,
        _client: &Client,
        resource: &ZwlrOutputConfigurationV1,
        request: zwlr_output_configuration_v1::Request,
        _handle: &DisplayHandle,
        data_init: &mut DataInit<'_, AndroidSeatRuntime>,
    ) {
        match request {
            zwlr_output_configuration_v1::Request::EnableHead { id, head: _ } => {
                log::info!("SmithayRuntime: wlr-output-management enable_head");
                data_init.init(
                    id,
                    ConfigHeadUserData {
                        tx: self.head_tx.clone(),
                    },
                );
            }
            zwlr_output_configuration_v1::Request::DisableHead { .. } => {
                log::info!("SmithayRuntime: wlr-output-management disable_head");
            }
            zwlr_output_configuration_v1::Request::Apply => {
                log::info!("SmithayRuntime: wlr-output-management apply");

                let mut new_mode: Option<(i32, i32, i32)> = None;
                let mut new_scale: Option<f64> = None;
                let mut new_position: Option<(i32, i32)> = None;

                for action in self.head_rx.try_iter() {
                    match action {
                        HeadAction::SetCustomMode(w, h, refresh) => {
                            log::info!("  -> set_custom_mode {}x{}@{}", w, h, refresh);
                            new_mode = Some((w, h, refresh));
                        }
                        HeadAction::SetPosition(x, y) => {
                            log::info!("  -> set_position {},{}", x, y);
                            new_position = Some((x, y));
                        }
                        HeadAction::SetTransform(t) => {
                            log::info!("  -> set_transform {} (ignored, fixed Normal)", t);
                        }
                        HeadAction::SetScale(s) => {
                            log::info!("  -> set_scale {}", s);
                            new_scale = Some(s);
                        }
                        HeadAction::SetAdaptiveSync(enabled) => {
                            log::info!("  -> set_adaptive_sync {} (ignored)", enabled);
                        }
                    }
                }

                // Determine effective resolution: use new if provided, otherwise current
                let (w, h) = match new_mode {
                    Some((w, h, _refresh)) => (w, h),
                    None => {
                        let current = state.output.current_mode().unwrap_or(smithay::output::Mode {
                            size: (1080, 2296).into(),
                            refresh: 60000,
                        });
                        (current.size.w, current.size.h)
                    }
                };

                log::info!(
                    "  -> applying mode {}x{} scale={:?} position={:?}",
                    w, h, new_scale, new_position,
                );

                state.update_output_mode(w, h, new_scale);
                // TODO: apply position (state.space.map_output) when multi-output is supported

                resource.succeeded();
            }
            zwlr_output_configuration_v1::Request::Test => {
                log::info!("SmithayRuntime: wlr-output-management test");

                let mut valid = true;
                for action in self.head_rx.try_iter() {
                    match action {
                        HeadAction::SetCustomMode(w, h, refresh) => {
                            if w <= 0 || h <= 0 || refresh <= 0 {
                                log::warn!("  -> test FAILED: invalid mode {}x{}@{}", w, h, refresh);
                                valid = false;
                            } else {
                                log::info!("  -> test set_custom_mode {}x{}@{} (valid)", w, h, refresh);
                            }
                        }
                        HeadAction::SetPosition(x, y) => {
                            log::info!("  -> test set_position {},{} (valid)", x, y);
                        }
                        HeadAction::SetTransform(t) => {
                            log::info!("  -> test set_transform {} (valid, ignored)", t);
                        }
                        HeadAction::SetScale(s) => {
                            if s <= 0.0 || s > 5.0 {
                                log::warn!("  -> test FAILED: invalid scale {}", s);
                                valid = false;
                            } else {
                                log::info!("  -> test set_scale {} (valid)", s);
                            }
                        }
                        HeadAction::SetAdaptiveSync(enabled) => {
                            log::info!("  -> test set_adaptive_sync {} (ignored)", enabled);
                        }
                    }
                }

                if valid {
                    resource.succeeded();
                } else {
                    resource.failed();
                }
            }
            zwlr_output_configuration_v1::Request::Destroy => {}
            _ => {}
        }
    }
}

// ── Head configuration dispatch ──────────────────────────────────────────────

#[cfg(feature = "smithay_android")]
impl smithay::wayland::Dispatch2<ZwlrOutputConfigurationHeadV1, AndroidSeatRuntime>
    for ConfigHeadUserData
{
    fn request(
        &self,
        _state: &mut AndroidSeatRuntime,
        _client: &Client,
        _resource: &ZwlrOutputConfigurationHeadV1,
        request: zwlr_output_configuration_head_v1::Request,
        _handle: &DisplayHandle,
        _data_init: &mut DataInit<'_, AndroidSeatRuntime>,
    ) {
        match request {
            zwlr_output_configuration_head_v1::Request::SetMode { mode: _ } => {
                log::info!("SmithayRuntime: wlr-output-management set_mode");
            }
            zwlr_output_configuration_head_v1::Request::SetCustomMode {
                width,
                height,
                refresh,
            } => {
                log::info!(
                    "SmithayRuntime: wlr-output-management set_custom_mode {}x{}@{}",
                    width,
                    height,
                    refresh
                );
                let _ = self
                    .tx
                    .send(HeadAction::SetCustomMode(width, height, refresh));
            }
            zwlr_output_configuration_head_v1::Request::SetPosition { x, y } => {
                log::info!(
                    "SmithayRuntime: wlr-output-management set_position {},{}",
                    x,
                    y
                );
                let _ = self.tx.send(HeadAction::SetPosition(x, y));
            }
            zwlr_output_configuration_head_v1::Request::SetTransform { transform } => {
                let val: i32 = match transform {
                    WEnum::Value(t) => match t {
                        WlTransform::Normal => 0,
                        WlTransform::_90 => 1,
                        WlTransform::_180 => 2,
                        WlTransform::_270 => 3,
                        WlTransform::Flipped => 4,
                        WlTransform::Flipped90 => 5,
                        WlTransform::Flipped180 => 6,
                        WlTransform::Flipped270 => 7,
                        _ => 0,
                    },
                    WEnum::Unknown(v) => v as i32,
                };
                log::info!(
                    "SmithayRuntime: wlr-output-management set_transform {}",
                    val
                );
                let _ = self.tx.send(HeadAction::SetTransform(val));
            }
            zwlr_output_configuration_head_v1::Request::SetScale { scale } => {
                log::info!("SmithayRuntime: wlr-output-management set_scale {}", scale);
                let _ = self.tx.send(HeadAction::SetScale(scale));
            }
            zwlr_output_configuration_head_v1::Request::SetAdaptiveSync { state: _ } => {
                log::info!("SmithayRuntime: wlr-output-management set_adaptive_sync");
                let _ = self.tx.send(HeadAction::SetAdaptiveSync(false));
            }
            _ => {}
        }
    }
}

// ── Delegate ──────────────────────────────────────────────────────────────────
// NOTE: delegate_dispatch2! is already called in runtime_state.rs
