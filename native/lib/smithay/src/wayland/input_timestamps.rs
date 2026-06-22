//! Implementation of the `input-timestamps-unstable-v1` protocol
//!
//! This is a Wayland protocol extension that allows clients to receive
//! high-resolution timestamps for input events (keyboard, pointer, touch).
//!
//! ## How to use it
//!
//! ### Initialization
//!
//! To initialize this implementation create the [`InputTimestampsManagerState`]
//! and store it inside your `State` struct.
//!
//! ```
//! use smithay::wayland::input_timestamps::InputTimestampsManagerState;
//!
//! # struct State { input_timestamps_state: InputTimestampsManagerState }
//! # let mut display = wayland_server::Display::<State>::new().unwrap();
//! let input_timestamps_state = InputTimestampsManagerState::new::<State>(&display.handle());
//!
//! smithay::delegate_dispatch2!(State);
//! ```
//!
//! ### Sending timestamps
//!
//! Whenever you process input events, call the appropriate method on the shared state
//! to send high-resolution timestamps to subscribed clients:
//!
//! ```no_run
//! # use smithay::wayland::input_timestamps::InputTimestampsManagerState;
//! # struct State { input_timestamps_state: InputTimestampsManagerState }
//! # fn handle_keyboard_event(state: &mut State) {
//! # let keyboard: wayland_server::protocol::wl_keyboard::WlKeyboard = todo!();
//! use std::time::{SystemTime, UNIX_EPOCH};
//!
//! let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
//! let tv_sec_hi = (now.as_secs() >> 32) as u32;
//! let tv_sec_lo = (now.as_secs() & 0xffffffff) as u32;
//! let tv_nsec = now.subsec_nanos();
//! state.input_timestamps_state.state().keyboard_timestamp(&keyboard, tv_sec_hi, tv_sec_lo, tv_nsec);
//! # }
//! ```

use std::sync::{Arc, Mutex};

use wayland_protocols::wp::input_timestamps::zv1::server::{
    zwp_input_timestamps_v1::{self, ZwpInputTimestampsV1},
    zwp_input_timestamps_manager_v1::{self, ZwpInputTimestampsManagerV1},
};
use wayland_server::{
    Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
    backend::{ClientId, GlobalId, ObjectId},
    protocol::{
        wl_keyboard::WlKeyboard,
        wl_pointer::WlPointer,
        wl_touch::WlTouch,
    },
};

use crate::wayland::{Dispatch2, GlobalDispatch2};

const MANAGER_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
enum InputTimestampDevice {
    Keyboard(ObjectId),
    Pointer(ObjectId),
    Touch(ObjectId),
}

/// User data for [`ZwpInputTimestampsV1`] objects.
#[derive(Debug)]
pub struct InputTimestampUserData {
    state: Arc<InputTimestampsState>,
}

/// Shared state for storing and sending input timestamps.
///
/// The compositor sends high-resolution timestamps alongside input events
/// by calling [`InputTimestampsState::keyboard_timestamp`],
/// [`InputTimestampsState::pointer_timestamp`], or
/// [`InputTimestampsState::touch_timestamp`].
#[derive(Debug, Default)]
pub struct InputTimestampsState {
    known_timestamps: Mutex<Vec<(InputTimestampDevice, ZwpInputTimestampsV1)>>,
}

impl InputTimestampsState {
    fn add_timestamp(&self, device: InputTimestampDevice, ts: ZwpInputTimestampsV1) {
        self.known_timestamps.lock().unwrap().push((device, ts));
    }

    /// Send a high-resolution timestamp for the given keyboard.
    ///
    /// All timestamp subscriptions associated with `keyboard` will receive
    /// the timestamp event.
    pub fn keyboard_timestamp(&self, keyboard: &WlKeyboard, tv_sec_hi: u32, tv_sec_lo: u32, tv_nsec: u32) {
        let timestamps = self.known_timestamps.lock().unwrap();
        for (device, ts) in timestamps.iter() {
            if matches!(device, InputTimestampDevice::Keyboard(id) if id == &keyboard.id()) {
                ts.timestamp(tv_sec_hi, tv_sec_lo, tv_nsec);
            }
        }
    }

    /// Send a high-resolution timestamp for the given pointer.
    pub fn pointer_timestamp(&self, pointer: &WlPointer, tv_sec_hi: u32, tv_sec_lo: u32, tv_nsec: u32) {
        let timestamps = self.known_timestamps.lock().unwrap();
        for (device, ts) in timestamps.iter() {
            if matches!(device, InputTimestampDevice::Pointer(id) if id == &pointer.id()) {
                ts.timestamp(tv_sec_hi, tv_sec_lo, tv_nsec);
            }
        }
    }

    /// Send a high-resolution timestamp for the given touch device.
    pub fn touch_timestamp(&self, touch: &WlTouch, tv_sec_hi: u32, tv_sec_lo: u32, tv_nsec: u32) {
        let timestamps = self.known_timestamps.lock().unwrap();
        for (device, ts) in timestamps.iter() {
            if matches!(device, InputTimestampDevice::Touch(id) if id == &touch.id()) {
                ts.timestamp(tv_sec_hi, tv_sec_lo, tv_nsec);
            }
        }
    }

    fn remove_timestamp(&self, object_id: &ObjectId) {
        self.known_timestamps
            .lock()
            .unwrap()
            .retain(|(_, ts)| ts.id() != *object_id);
    }
}

/// Global user-data for the input timestamps manager.
///
/// Carries a reference to the shared [`InputTimestampsState`].
#[derive(Debug, Clone)]
pub struct InputTimestampsManagerData {
    state: Arc<InputTimestampsState>,
}

/// State for the [`ZwpInputTimestampsManagerV1`] global.
#[derive(Debug, Clone)]
pub struct InputTimestampsManagerState {
    global: GlobalId,
    state: Arc<InputTimestampsState>,
}

impl InputTimestampsManagerState {
    /// Create a new [`ZwpInputTimestampsManagerV1`] global.
    pub fn new<D>(display: &DisplayHandle) -> Self
    where
        D: GlobalDispatch<ZwpInputTimestampsManagerV1, InputTimestampsManagerData>,
        D: Dispatch<ZwpInputTimestampsManagerV1, InputTimestampsManagerData>,
        D: Dispatch<ZwpInputTimestampsV1, InputTimestampUserData>,
        D: 'static,
    {
        let state = Arc::new(InputTimestampsState::default());
        let data = InputTimestampsManagerData { state: state.clone() };
        let global = display.create_global::<D, ZwpInputTimestampsManagerV1, _>(MANAGER_VERSION, data);
        Self { global, state }
    }

    /// Access the shared state for sending timestamps.
    pub fn state(&self) -> &InputTimestampsState {
        &self.state
    }

    /// The global id of this manager.
    pub fn global(&self) -> GlobalId {
        self.global.clone()
    }
}

impl<D> GlobalDispatch2<ZwpInputTimestampsManagerV1, D> for InputTimestampsManagerData
where
    D: Dispatch<ZwpInputTimestampsManagerV1, InputTimestampsManagerData>,
    D: 'static,
{
    fn bind(
        &self,
        _state: &mut D,
        _dh: &DisplayHandle,
        _client: &Client,
        resource: New<ZwpInputTimestampsManagerV1>,
        _data_init: &mut DataInit<'_, D>,
    ) {
        _data_init.init(resource, InputTimestampsManagerData {
            state: self.state.clone(),
        });
    }
}

impl<D> Dispatch2<ZwpInputTimestampsManagerV1, D> for InputTimestampsManagerData
where
    D: Dispatch<ZwpInputTimestampsV1, InputTimestampUserData>,
    D: 'static,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _manager: &ZwpInputTimestampsManagerV1,
        request: zwp_input_timestamps_manager_v1::Request,
        _dh: &DisplayHandle,
        data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwp_input_timestamps_manager_v1::Request::GetKeyboardTimestamps { id, keyboard } => {
                let ts = data_init.init(
                    id,
                    InputTimestampUserData {
                        state: self.state.clone(),
                    },
                );
                self.state.add_timestamp(InputTimestampDevice::Keyboard(keyboard.id()), ts);
            }
            zwp_input_timestamps_manager_v1::Request::GetPointerTimestamps { id, pointer } => {
                let ts = data_init.init(
                    id,
                    InputTimestampUserData {
                        state: self.state.clone(),
                    },
                );
                self.state.add_timestamp(InputTimestampDevice::Pointer(pointer.id()), ts);
            }
            zwp_input_timestamps_manager_v1::Request::GetTouchTimestamps { id, touch } => {
                let ts = data_init.init(
                    id,
                    InputTimestampUserData {
                        state: self.state.clone(),
                    },
                );
                self.state.add_timestamp(InputTimestampDevice::Touch(touch.id()), ts);
            }
            zwp_input_timestamps_manager_v1::Request::Destroy => {}
            _ => unreachable!(),
        }
    }
}

impl<D> Dispatch2<ZwpInputTimestampsV1, D> for InputTimestampUserData
where
    D: 'static,
{
    fn request(
        &self,
        _state: &mut D,
        _client: &Client,
        _timestamp: &ZwpInputTimestampsV1,
        request: zwp_input_timestamps_v1::Request,
        _dh: &DisplayHandle,
        _data_init: &mut DataInit<'_, D>,
    ) {
        match request {
            zwp_input_timestamps_v1::Request::Destroy => {}
            _ => unreachable!(),
        }
    }

    fn destroyed(&self, _state: &mut D, _client: ClientId, resource: &ZwpInputTimestampsV1) {
        self.state.remove_timestamp(&resource.id());
    }
}
