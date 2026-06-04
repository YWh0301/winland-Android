# Master Plan — Fix All BUG_TRACKER.md Issues

## Strategy

Phase | Focus | Issues | Status
------|-------|--------|-------
1 | Safety & Crashes | R1, W1, W3, S1, S2, S4 | **DONE** (commit 5f33e49)
2 | XWayland Stability | X3(SKIP), X4(SKIP), X6, X7, X8, X10 | **DONE** (commit 0559ec0)
3 | Touch & Gestures | T2, T3, T4, T5, T6, T7 | Pending
4 | Trackpad | P1, P2, P3, P4, P5, P6 | Pending
5 | Window Management | WM1, WM2, WM3, WM4, WM5, WM6, WM7, WM8, WM9 | Pending
6 | Pointers & Cursor | M2, M3, M4, M5, M6 | Pending
7 | Wayland Protocols | W2, W5, W6, W7, W8 | Pending
8 | Rendering | R2, R3, R4, R5, R7 | Pending
9 | Windows & Shell | N1, N2, N3, N4 | Pending

---

## Phase 1 — Safety & Crashes

### R1 — Unsafe SHM bounds check
**File**: `seat.rs:917,988,1029,1138`
**Fix**: Add `len` validation before `from_raw_parts`. Ensure `offset + stride * height ≤ len`.

### W1 — Lock unwrap on poisoned mutex
**File**: `handlers.rs:375`
**Fix**: Replace `lock().unwrap()` with `lock().ok().and_then(|guard| ...)` or log + skip.

### W3 — Raw FD ownership
**File**: `selection.rs:56,105`
**Fix**: Wrap `OwnedFd` in a struct that implements `Drop` to prevent FD leaks.

### S1 — Full buffer copy every frame
**File**: `seat.rs:929`
**Fix**: Cache last buffer ID per surface; only re-read if buffer changed.

### S2 — Eager render on every event
**File**: `input_router.rs:367`
**Fix**: Coalesce render requests with a flag + deferred render (schedule on idle).

### S4 — Unbounded channel
**File**: `seat.rs:242`
**Fix**: Switch to bounded channel or drop stale frames if compositor is behind.

---

## Phase 2 — XWayland Stability

### X3 — Zombie state on XWayland failure
**File**: `server.rs:271`
**SKIP** — يمس آلية تشغيل XWayland (launch mechanism). التشغيل يدوي من RTF.

### X4 — Socket race condition
**File**: `server.rs:245-258`
**SKIP** — يمس socket readiness/launch.

### X6 — Single xwm_id assumption
**File**: `x11.rs:55-58`
**Fix**: Store `HashMap<XwmId, X11Wm>` instead of single `Option<X11Wm>`.

### X7 — Gesture state mismatch on unmapped
**File**: `x11.rs:144-146`
**Fix**: Add guard: `if self.gesture_surface.as_ref() == Some(&wl)`.

### X8 — Configure request vs compositor position
**File**: `x11.rs` configure_request handler
**Fix**: Ignore position from configure_request for XWayland windows (compositor is authoritative). Only accept size changes.

### X10 — Dialog window type
**File**: `x11.rs` map_window_request
**Fix**: Read `_NET_WM_WINDOW_TYPE` atom; if `_NET_WM_WINDOW_TYPE_DIALOG`, center on parent or screen.

---

## Phase 3 — Touch & Gestures

### T2 — Invisible titlebar
**File**: `input_router.rs:231`, `seat.rs:1076` render_all
**Fix**: Render a semi-transparent titlebar strip (24px tall) above each window. Draw close/minimize/maximize button indicators.

### T3 — Multi-touch finger tracking
**File**: `input.rs:44-46`, `input_router.rs`
**Fix**: Track individual finger IDs throughout gesture lifecycle. Don't merge to centroid unless 2+ fingers.

### T4 — primary_touch_id cleanup
**File**: `input_router.rs`
**Fix**: Clear `primary_touch_id` on TouchUp, TouchCancel, and gesture timeout.

### T5 — Hardcoded pixel zones
**File**: `input_router.rs:232-248`
**Fix**: Express offsets as logical pixels scaled by `output_scale` + `ui_scale`.

### T6 — Two-finger right-click
**File**: `input_router.rs`
**Fix**: On 2-finger tap (both Down then Up within 300ms), emit right-click (button 0x111).

### T7 — Long-press right-click
**File**: `input_router.rs`
**Fix**: In Touch mode, if finger holds still >500ms within 24px titlebar zone → emit right-click.

---

## Phase 4 — Trackpad

### P1 — Absolute→relative conversion
**File**: `input_router.rs:394-420`
**Fix**: Trackpad already gets absolute coords from Android; use raw deltas, not absolute→relative extrapolation.

### P2 — Drag release on finger lift
**File**: `input_router.rs:431-445` + handle_trackpad_up
**Fix**: On trackpad up, if `trackpad_dragging` is true, send `ButtonState::Released` + `frame()`.

### P3 — Noise gate too aggressive
**File**: `input_router.rs:421-423`
**Fix**: Lower threshold to 0.1 or make configurable.

### P4 — Tap click
**File**: `input_router.rs`
**Fix**: On finger down-up within 300ms without significant movement, send left-click (button 0x110).

### P5 — Acceleration curve
**File**: `input_router.rs`
**Fix**: Apply non-linear curve: `dx * (1 + k * log(1 + |dx|))`.

### P6 — Two-finger scroll
**File**: `input_router.rs`
**Fix**: Track two finger positions; compute scroll delta from centroid movement; emit `AxisFrame`.

---

## Phase 5 — Window Management

### WM1 — Configure race
**File**: `input_router.rs:376-390`
**Fix**: In `dispatch_touch_up_gesture`, compare final position with X11 window's current geometry. If client moved itself since gesture start, skip our configure.

### WM2 — Active/inactive visual state
**File**: `shell.rs:44-46`, `seat.rs`
**Fix**: Dim inactive windows by reducing brightness or adding overlay. Track active window in `AndroidSeatRuntime`.

### WM3 — Focus candidate on unmapped
**File**: `x11.rs:150`
**Fix**: In `choose_focus_candidate`, sort by MRU (most recently focused) instead of stacking order.

### WM4 — Maximize restore off-screen
**File**: `seat.rs:703-704`
**Fix**: Clamp restore position to `(0, reserved_top)` ≤ pos ≤ `(max_x, max_y)`.

### WM5 — Minimize animation
**File**: `seat.rs:669-694`
**Fix**: Animate window shrinking/sliding off-screen using render_all position interpolation.

### WM6 — Resize/titlebar zone overlap
**File**: `input_router.rs:257-258 vs 231`
**Fix**: Titlebar zone (24px) takes priority over resize zone (20px) at top edge. Only test resize for y > fy.

### WM7 — Keyboard shortcuts
**File**: `input_router.rs` keyboard handling
**Fix**: Add Alt+F4 → close_surface(focused), Alt+Tab → cycle windows, etc.

### WM8 — Snap/tiling
**File**: `input_router.rs`
**Fix**: On move completion, if window dragged within 20% of screen edge, snap to half-screen.

### WM9 — Titlebar context menu
**File**: `input_router.rs` titlebar right-click
**Fix**: On right-click in titlebar zone, show menu: minimize, maximize, close.

---

## Phase 6 — Pointers & Cursor

### M2 — Cursor hotspot (0,0)
**File**: `seat.rs:1106-1107`
**Fix**: Default hotspot to center of cursor surface `(w/2, h/2)` if client doesn't set it.

### M3 — Hidden cursor renders surface cursor
**File**: `seat.rs:1088`
**Fix**: `CursorImageStatus::Hidden` should skip ALL cursor rendering, not just named fallback.

### M4 — Hardcoded fallback cursor
**File**: `seat.rs:1184-1222`
**Fix**: Load PNG cursor from assets or generate via FreeType.

### M5 — cursor-shape-v1 protocol
**File**: `handlers.rs`, `seat.rs`
**Fix**: Implement `wp_cursor_shape_manager_v1`. Map stock cursor names to actual shapes.

### M6 — HiDPI cursor
**File**: `seat.rs`
**Fix**: Scale cursor buffer by output_scale before rendering.

---

## Phase 7 — Wayland Protocols

### W2 — WaylandClientState missing
**File**: `handlers.rs:236`
**Fix**: In `seat_injector.rs` or `server.rs`, set a fallback `WaylandClientState` on the XWayland client before any handler fires.

### W5 — Popup dismissal on wrong window
**File**: `input_router.rs:204`
**Fix**: Use the popup's own geometry tree to detect outside-tap, not `element_under()`.

### W6 — Data device drag-and-drop
**File**: New file
**Fix**: Wire up `wl_data_device_manager` → `WlDataDevice`, `WlDataSource`, `WlDataOffer`. Connect to DnD icon rendering.

### W7 — Relative pointer
**File**: `handlers.rs`
**Fix**: Wire up `zwp_relative_pointer_manager_v1` → emit relative motion events from Mouse mode.

### W8 — Pointer constraints
**File**: `input_router.rs:908-910`
**Fix**: Implement `set_pointer_constraint` + `lock` + `confine` with cursor locking region.

---

## Phase 8 — Rendering

### R2 — Non-SHM fallback
**File**: `seat.rs:912`
**Fix**: For non-SHM buffers (EGL/DMA-BUF — though DMA-BUF is banned), read via `wl_buffer` → `get_dmabuf` → map. If unavailable, skip with clear error.

### R3 — Damage tracking
**File**: `seat.rs:871`
**Fix**: Track `SurfaceAttributes::damage` per surface. Only re-read damaged regions. Only composite surfaces with pending damage.

### R4 — GLES program reuse
**File**: `smithay_backend.rs:28-51`
**Fix**: Compile shader program once in `bind_native_window` full init path; store in `AndroidSmithayState.gl_program`.

### R5 — Surface scale mismatch
**File**: `smithay_backend.rs:97-100`
**Fix**: When uploading texture, divide buffer dimensions by surface_scale for NDC. Verify exact pixel mapping.

### R7 — Partial-update / dirty regions
**File**: `seat.rs:924-935`
**Fix**: Instead of full row copy, only copy `info.damage` rects from SHM buffer. Compose damaged regions only.

---

## Phase 9 — Windows & Shell

### N1 — Cow surface mismatch on remap
**File**: `shell.rs:28-62`
**Fix**: In `unmap_window`, remove from `wl_to_window` map. On re-map, insert fresh. Don't cache `Cow::Owned` across remaps.

### N2 — Stale X11Surface in Window clones
**File**: `x11.rs:119`, `seat.rs:641`
**Fix**: `Window::new_x11_window` moves `X11Surface`. After that, use the `Window` ref; don't re-create `X11Surface`.

### N3 — Unmanaged surface position
**File**: `seat.rs:1061`
**Fix**: Track original `(x, y)` from `new_override_redirect_window`. Store in position map alongside `unmanaged_surfaces`.

### N4 — XDG popup relative positioning
**File**: `seat.rs:966-1003`
**Fix**: Use `PopupManager::popups_for_surface` which already returns absolute position. Verify popup_loc is correct relative to parent.
