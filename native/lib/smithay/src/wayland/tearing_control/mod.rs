//! Implementation of the `wp_tearing_control` protocol
//!
//! ## How to use it
//!
//! ### Initialization
//!
//! To initialize this implementation create the [`TearingControlState`] and store it inside your `State` struct.
//!
//! ```
//! use smithay::wayland::compositor;
//! use smithay::wayland::tearing_control::{TearingControlCachedState, TearingControlState};
//!
//! # struct State { tearing_control_state: TearingControlState }
//! # let mut display = wayland_server::Display::<State>::new().unwrap();
//! let tearing_control_state = TearingControlState::new::<State>(&display.handle());
//!
//! smithay::delegate_dispatch2!(State);
//! ```
//!
//! ### Use the tearing control state
//!
//! The presentation hint set by a client is double-buffered and can be retrieved during
//! surface commit via the compositor cached state:
//!
//! ```no_run
//! use smithay::wayland::compositor;
//! use smithay::wayland::tearing_control::{TearingControlCachedState, PresentationHint};
//!
//! # let surface: wayland_server::protocol::wl_surface::WlSurface = todo!();
//! compositor::with_states(&surface, |states| {
//!     let hint = states.cached_state.get::<TearingControlCachedState>().current().hint;
//!     match hint {
//!         PresentationHint::Vsync => { /* tear-free presentation */ }
//!         PresentationHint::Async => { /* tearing allowed */ }
//!     }
//! });
//! ```

use std::sync::{
    Mutex,
    atomic::{self, AtomicBool},
};

use wayland_protocols::wp::tearing_control::v1::server::{
    wp_tearing_control_manager_v1::WpTearingControlManagerV1,
    wp_tearing_control_v1,
};
use wayland_server::{
    Dispatch, DisplayHandle, GlobalDispatch, Resource, Weak, backend::GlobalId,
    protocol::wl_surface::WlSurface,
};

use super::compositor::Cacheable;

use crate::wayland::GlobalData;

mod dispatch;

/// Hint for the preferred presentation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PresentationHint {
    /// Synchronized to vertical blanking period (tear-free)
    #[default]
    Vsync,
    /// Asynchronous presentation (tearing acceptable)
    Async,
}

impl PresentationHint {
    fn from_wp(val: wp_tearing_control_v1::PresentationHint) -> Self {
        match val {
            wp_tearing_control_v1::PresentationHint::Vsync => PresentationHint::Vsync,
            wp_tearing_control_v1::PresentationHint::Async => PresentationHint::Async,
            _ => PresentationHint::Vsync,
        }
    }
}

/// Data associated with WlSurface
/// Represents the client pending state for tearing control
#[derive(Debug, Clone, Copy, Default)]
pub struct TearingControlCachedState {
    /// The presentation hint for this surface
    pub hint: PresentationHint,
}

impl Cacheable for TearingControlCachedState {
    fn commit(&mut self, _dh: &DisplayHandle) -> Self {
        std::mem::take(self)
    }

    fn merge_into(self, into: &mut Self, _dh: &DisplayHandle) {
        *into = self;
    }
}

impl std::ops::Deref for TearingControlCachedState {
    type Target = PresentationHint;

    fn deref(&self) -> &Self::Target {
        &self.hint
    }
}

#[derive(Debug)]
struct TearingControlSurfaceData {
    is_resource_attached: AtomicBool,
}

impl TearingControlSurfaceData {
    fn new() -> Self {
        Self {
            is_resource_attached: AtomicBool::new(false),
        }
    }

    fn set_is_resource_attached(&self, is_attached: bool) {
        self.is_resource_attached
            .store(is_attached, atomic::Ordering::Release)
    }

    fn is_resource_attached(&self) -> bool {
        self.is_resource_attached.load(atomic::Ordering::Acquire)
    }
}

/// User data of [WpTearingControlV1] object
#[derive(Debug)]
pub struct TearingControlUserData(Mutex<Weak<WlSurface>>);

impl TearingControlUserData {
    fn new(surface: WlSurface) -> Self {
        Self(Mutex::new(surface.downgrade()))
    }

    fn wl_surface(&self) -> Option<WlSurface> {
        self.0.lock().unwrap().upgrade().ok()
    }
}

/// Delegate type for [WpTearingControlManagerV1] global.
#[derive(Debug)]
pub struct TearingControlState {
    global: GlobalId,
}

impl TearingControlState {
    /// Register new [WpTearingControlManagerV1] global
    pub fn new<D>(display: &DisplayHandle) -> TearingControlState
    where
        D: GlobalDispatch<WpTearingControlManagerV1, GlobalData>
            + Dispatch<WpTearingControlManagerV1, GlobalData>
            + Dispatch<wp_tearing_control_v1::WpTearingControlV1, TearingControlUserData>
            + 'static,
    {
        let global = display.create_global::<D, WpTearingControlManagerV1, _>(1, GlobalData);

        TearingControlState { global }
    }

    /// Returns the [WpTearingControlManagerV1] global id
    pub fn global(&self) -> GlobalId {
        self.global.clone()
    }
}
