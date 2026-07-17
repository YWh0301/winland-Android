use nix::sys::socket::{connect, recvmsg, sendmsg, AddressFamily, ControlMessage, MsgFlags, SockFlag, SockType, UnixAddr};
use std::io::{IoSlice, IoSliceMut};
use std::os::fd::{AsRawFd, OwnedFd};

fn cpu_read_dmabuf(fd: i32, offset: usize, stride: usize, width: usize, height: usize) -> Option<Vec<u8>> {
    let map_len = offset + stride * height;
    let ptr = unsafe {
        libc::mmap(std::ptr::null_mut(), map_len,
                   libc::PROT_READ, libc::MAP_SHARED, fd, 0)
    };
    if ptr == libc::MAP_FAILED {
        log::warn!("dmabuf CPU fallback mmap FAILED fd={} map_len={}", fd, map_len);
        return None;
    }
    let mut pixels = Vec::with_capacity(width * height * 4);
    unsafe {
        let base = (ptr as *const u8).add(offset);
        for row in 0..height {
            let src = base.add(row * stride);
            for col in 0..width {
                let p = src.add(col * 4);
                pixels.push(*p);           // R
                pixels.push(*p.add(1));    // G
                pixels.push(*p.add(2));    // B
                pixels.push(255);          // A (force opaque)
            }
        }
        libc::munmap(ptr, map_len);
    }
    if pixels.is_empty() { None } else { Some(pixels) }
}

/// Render multiple surfaces in a single GLES pass with proper compositing.
/// The caller owns the state (on the compositor thread) and passes it by reference.
pub(crate) fn composite_multi(state: &mut AndroidSmithayState, surfaces: &[RenderItem]) {
    if surfaces.is_empty() {
        return;
    }

    let n_shm = surfaces.iter().filter(|s| matches!(s, RenderItem::Shm { .. })).count();
    let n_dmabuf = surfaces.iter().filter(|s| matches!(s, RenderItem::DmaBuf { .. })).count();
    log::info!("composite_multi: {} surfaces ({} SHM, {} DMA-BUF), render_active={}",
        surfaces.len(), n_shm, n_dmabuf,
        crate::android::backend::wayland::engine_timing::is_rendering_active());

    if !crate::android::backend::wayland::engine_timing::is_rendering_active() {
        log::warn!("composite_multi: rendering not active, skipping");
        return;
    }

    let (display, surface, context, physical_sw, physical_sh, logical_sw, logical_sh) = {
        let (display, surface, context) = match (state.egl_display, state.egl_surface, state.egl_context) {
            (Some(d), Some(s), Some(c)) => (d, s, c),
            _ => {
                log::warn!("composite_multi: egl_display/surface/context not ready");
                return;
            }
        };

        let physical_sw = state.surface_size.0 as f32;
        let physical_sh = state.surface_size.1 as f32;
        if physical_sw <= 0.0 || physical_sh <= 0.0 {
            log::warn!("composite_multi: surface_size {}x{} invalid", state.surface_size.0, state.surface_size.1);
            return;
        }
        let scale = state.current_scale.max(0.1);
        let logical_sw = physical_sw / scale;
        let logical_sh = physical_sh / scale;

        if state.gl_program.is_none() {
            let vs_src = "attribute vec2 aPos; attribute vec2 aTex; varying vec2 vTex; void main(){ vTex=aTex; gl_Position=vec4(aPos,0.0,1.0); }";
            let fs_src = "precision mediump float; varying vec2 vTex; uniform sampler2D uTex; void main(){ vec4 c=texture2D(uTex,vTex); gl_FragColor=vec4(c.b,c.g,c.r,1.0); }";
            let fs_cursor_src = "precision mediump float; varying vec2 vTex; uniform sampler2D uTex; void main(){ vec4 c=texture2D(uTex,vTex); gl_FragColor=vec4(c.b,c.g,c.r,c.a); }";

            unsafe {
                let vs = gl::CreateShader(gl::VERTEX_SHADER);
                let c_vs = std::ffi::CString::new(vs_src).unwrap();
                gl::ShaderSource(vs, 1, &c_vs.as_ptr(), std::ptr::null());
                gl::CompileShader(vs);

                let fs = gl::CreateShader(gl::FRAGMENT_SHADER);
                let c_fs = std::ffi::CString::new(fs_src).unwrap();
                gl::ShaderSource(fs, 1, &c_fs.as_ptr(), std::ptr::null());
                gl::CompileShader(fs);
                let program = gl::CreateProgram();
                gl::AttachShader(program, vs);
                gl::AttachShader(program, fs);
                gl::BindAttribLocation(program, 0, std::ffi::CString::new("aPos").unwrap().as_ptr());
                gl::BindAttribLocation(program, 1, std::ffi::CString::new("aTex").unwrap().as_ptr());
                gl::LinkProgram(program);
                state.gl_program = Some(program as u32);
                gl::DeleteShader(fs);

                let fs_cursor = gl::CreateShader(gl::FRAGMENT_SHADER);
                let c_fs_cursor = std::ffi::CString::new(fs_cursor_src).unwrap();
                gl::ShaderSource(fs_cursor, 1, &c_fs_cursor.as_ptr(), std::ptr::null());
                gl::CompileShader(fs_cursor);
                let cursor_program = gl::CreateProgram();
                gl::AttachShader(cursor_program, vs);
                gl::AttachShader(cursor_program, fs_cursor);
                gl::BindAttribLocation(cursor_program, 0, std::ffi::CString::new("aPos").unwrap().as_ptr());
                gl::BindAttribLocation(cursor_program, 1, std::ffi::CString::new("aTex").unwrap().as_ptr());
                gl::LinkProgram(cursor_program);
                state.gl_cursor_program = Some(cursor_program as u32);
                gl::DeleteShader(fs_cursor);
                gl::DeleteShader(vs);
            }
        }

        (display, surface, context, physical_sw, physical_sh, logical_sw, logical_sh)
    };

    unsafe {
        if eglMakeCurrent(display, surface, surface, context) == egl::FALSE {
            log::warn!("composite_multi: eglMakeCurrent failed (egl_error=0x{:x})", eglGetError());
            return;
        }

        log::info!("composite_multi: eglMakeCurrent OK, viewport {}x{}",
            physical_sw as i32, physical_sh as i32);

        gl::Viewport(0, 0, physical_sw as i32, physical_sh as i32);
        gl::ClearColor(0.12, 0.12, 0.18, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);

        for item in surfaces {
            let is_dmabuf = matches!(item, RenderItem::DmaBuf { .. });
            if item.is_cursor() {
                let prog = if is_dmabuf {
                    state.gl_cursor_dmabuf_program
                } else {
                    state.gl_cursor_program
                };
                if let Some(prog) = prog {
                    gl::UseProgram(prog);
                }
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            } else {
                let prog = if is_dmabuf {
                    state.gl_dmabuf_program
                } else {
                    state.gl_program
                };
                if let Some(prog) = prog {
                    gl::UseProgram(prog);
                }
                gl::Disable(gl::BLEND);
            }

            let (_item_w, _item_h, item_x, item_y, item_scale) = match *item {
                RenderItem::Shm { width, height, x, y, scale, .. } => (width, height, x, y, scale),
                RenderItem::DmaBuf { width, height, x, y, scale, .. } => (width, height, x, y, scale),
            };

            match *item {
                RenderItem::Shm { ref pixels, width, height, .. } => {
                    let mut tex: gl::types::GLuint = 0;
                    gl::GenTextures(1, &mut tex);
                    gl::BindTexture(gl::TEXTURE_2D, tex);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as gl::types::GLint);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as gl::types::GLint);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as gl::types::GLint);
                    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as gl::types::GLint);
                    gl::TexImage2D(
                        gl::TEXTURE_2D, 0, gl::RGBA as gl::types::GLint,
                        width, height, 0, gl::RGBA, gl::UNSIGNED_BYTE,
                        pixels.as_ptr() as *const std::ffi::c_void,
                    );
                    let gle = gl::GetError();
                    if gle != gl::NO_ERROR {
                        log::warn!("SHM: glTexImage2D error=0x{:x} ({}x{})", gle, width, height);
                    }

                    let logical_w = width as f32 / item_scale;
                    let logical_h = height as f32 / item_scale;
                    let x0 = (item_x as f32 / logical_sw) * 2.0 - 1.0;
                    let y0 = 1.0 - (item_y as f32 / logical_sh) * 2.0;
                    let x1 = ((item_x as f32 + logical_w) / logical_sw) * 2.0 - 1.0;
                    let y1 = 1.0 - ((item_y as f32 + logical_h) / logical_sh) * 2.0;

                    let verts: [f32; 16] = [
                        x0, y1, 0.0, 1.0,
                        x1, y1, 1.0, 1.0,
                        x0, y0, 0.0, 0.0,
                        x1, y0, 1.0, 0.0,
                    ];

                    let stride = (4 * std::mem::size_of::<f32>()) as i32;
                    gl::EnableVertexAttribArray(0);
                    gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stride, verts.as_ptr() as *const std::ffi::c_void);
                    gl::EnableVertexAttribArray(1);
                    gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, verts.as_ptr().add(2) as *const std::ffi::c_void);
                    gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
                    let gle2 = gl::GetError();
                    if gle2 != gl::NO_ERROR {
                        log::warn!("SHM: glDrawArrays error=0x{:x} ({}x{} x0={} y0={})", gle2, width, height, x0, y0);
                    }
                    gl::DisableVertexAttribArray(0);
                    gl::DisableVertexAttribArray(1);
                    gl::DeleteTextures(1, &tex);
                }
                RenderItem::DmaBuf { ref fd, fourcc, modifier, offset, stride, width, height, .. } => {
                    if offset > 0 {
                        log::debug!("dmabuf: KGSL path detected fd={} offset={} (gem_handle={})",
                                    fd.as_raw_fd(), offset, offset >> 12);
                    }

                    let egl_ok = state.egl_create_image_khr.is_some()
                        && state.egl_destroy_image_khr.is_some()
                        && state.gl_egl_image_target_texture_2d_oes.is_some();

                    // Try EGL import first if available
                    if egl_ok {
                        let (Some(create_image), Some(destroy_image), Some(target_texture)) =
                            (state.egl_create_image_khr, state.egl_destroy_image_khr, state.gl_egl_image_target_texture_2d_oes)
                        else {
                            unreachable!(); // checked above
                        };

                        let fourcc_val = fourcc as i32;
                        let is_valid_modifier = modifier != 0 && modifier != 0x00FFFFFFFFFFFFFF;
                        let has_mods = state.has_dmabuf_import_modifiers && is_valid_modifier;
                        let attribs = if has_mods {
                            vec![
                                EGL_LINUX_DRM_FOURCC_EXT, fourcc_val,
                                EGL_DMA_BUF_PLANE0_FD_EXT, fd.as_raw_fd() as i32,
                                EGL_DMA_BUF_PLANE0_OFFSET_EXT, i32::try_from(offset).unwrap_or(0),
                                EGL_DMA_BUF_PLANE0_PITCH_EXT, stride as i32,
                                EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT, modifier as i32,
                                EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT, (modifier >> 32) as i32,
                                EGL_IMAGE_PRESERVED_KHR, 1,
                                EGL_NONE,
                            ]
                        } else {
                            vec![
                                EGL_LINUX_DRM_FOURCC_EXT, fourcc_val,
                                EGL_DMA_BUF_PLANE0_FD_EXT, fd.as_raw_fd() as i32,
                                EGL_DMA_BUF_PLANE0_OFFSET_EXT, i32::try_from(offset).unwrap_or(0),
                                EGL_DMA_BUF_PLANE0_PITCH_EXT, stride as i32,
                                EGL_IMAGE_PRESERVED_KHR, 1,
                                EGL_NONE,
                            ]
                        };

                        let egl_img = create_image(display, std::ptr::null_mut(), EGL_LINUX_DMA_BUF_EXT, std::ptr::null_mut(), attribs.as_ptr());
                        if !egl_img.is_null() {
                            let mut tex: gl::types::GLuint = 0;
                            gl::GenTextures(1, &mut tex);
                            gl::BindTexture(gl::TEXTURE_2D, tex);
                            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as gl::types::GLint);
                            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as gl::types::GLint);
                            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as gl::types::GLint);
                            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as gl::types::GLint);
                            target_texture(gl::TEXTURE_2D, egl_img);
                            destroy_image(display, egl_img);

                            let logical_w = width as f32 / item_scale;
                            let logical_h = height as f32 / item_scale;
                            let x0 = (item_x as f32 / logical_sw) * 2.0 - 1.0;
                            let y0 = 1.0 - (item_y as f32 / logical_sh) * 2.0;
                            let x1 = ((item_x as f32 + logical_w) / logical_sw) * 2.0 - 1.0;
                            let y1 = 1.0 - ((item_y as f32 + logical_h) / logical_sh) * 2.0;

                            let verts: [f32; 16] = [
                                x0, y1, 0.0, 1.0,
                                x1, y1, 1.0, 1.0,
                                x0, y0, 0.0, 0.0,
                                x1, y0, 1.0, 0.0,
                            ];

                            let vert_stride = (4 * std::mem::size_of::<f32>()) as i32;
                            gl::EnableVertexAttribArray(0);
                            gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, vert_stride, verts.as_ptr() as *const std::ffi::c_void);
                            gl::EnableVertexAttribArray(1);
                            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, vert_stride, verts.as_ptr().add(2) as *const std::ffi::c_void);
                            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
                            gl::DisableVertexAttribArray(0);
                            gl::DisableVertexAttribArray(1);
                            gl::DeleteTextures(1, &tex);
                            log::info!("DmaBuf: EGL imported {}x{}", width, height);
                            continue;
                        }
                        log::warn!("DmaBuf: eglCreateImageKHR failed, trying CPU fallback");
                    }

                    // CPU mmap fallback (EGL failed or unavailable)
                    let w = width as usize;
                    let h = height as usize;
                    let s = stride as usize;
                    let o = offset as usize;
                    if let Some(pixels) = cpu_read_dmabuf(
                        fd.as_raw_fd(), o, s, w, h,
                    ) {
                        let mut tex: gl::types::GLuint = 0;
                        gl::GenTextures(1, &mut tex);
                        gl::BindTexture(gl::TEXTURE_2D, tex);
                        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as gl::types::GLint);
                        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as gl::types::GLint);
                        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as gl::types::GLint);
                        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as gl::types::GLint);
                        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as i32,
                                       width, height, 0, gl::RGBA,
                                       gl::UNSIGNED_BYTE, pixels.as_ptr() as *const std::ffi::c_void);

                        let logical_w = width as f32 / item_scale;
                        let logical_h = height as f32 / item_scale;
                        let x0 = (item_x as f32 / logical_sw) * 2.0 - 1.0;
                        let y0 = 1.0 - (item_y as f32 / logical_sh) * 2.0;
                        let x1 = ((item_x as f32 + logical_w) / logical_sw) * 2.0 - 1.0;
                        let y1 = 1.0 - ((item_y as f32 + logical_h) / logical_sh) * 2.0;

                        let verts: [f32; 16] = [
                            x0, y1, 0.0, 1.0,
                            x1, y1, 1.0, 1.0,
                            x0, y0, 0.0, 0.0,
                            x1, y0, 1.0, 0.0,
                        ];

                        let vert_stride = (4 * std::mem::size_of::<f32>()) as i32;
                        gl::EnableVertexAttribArray(0);
                        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, vert_stride, verts.as_ptr() as *const std::ffi::c_void);
                        gl::EnableVertexAttribArray(1);
                        gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, vert_stride, verts.as_ptr().add(2) as *const std::ffi::c_void);
                        gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);
                        gl::DisableVertexAttribArray(0);
                        gl::DisableVertexAttribArray(1);
                        gl::DeleteTextures(1, &tex);
                        log::info!("DmaBuf: CPU fallback rendered {}x{} via mmap", width, height);
                        continue;
                    }
                    log::warn!("DmaBuf: fallback failed, skipping");
                    continue;
                }
            }
        }

        gl::UseProgram(0);

        let swap_ok = eglSwapBuffers(display, surface);
        log::info!("composite_multi: eglSwapBuffers -> {} (egl_error=0x{:x})", swap_ok, eglGetError());
        let _ = eglMakeCurrent(display, egl::NO_SURFACE, egl::NO_SURFACE, egl::NO_CONTEXT);
    }

    state.frames_presented += 1;
    log::info!("composite_multi: done, frames_presented={}", state.frames_presented);
}

pub fn render_background_tick(state: &AndroidSmithayState) {
    if state.frames_presented > 0 {
        return;
    }
    if !crate::android::backend::wayland::engine_timing::is_rendering_active() {
        return;
    }

    let (display, surface, context) = match (state.egl_display, state.egl_surface, state.egl_context) {
        (Some(d), Some(s), Some(c)) => (d, s, c),
        _ => return,
    };
    let (width, height) = state.surface_size;

    unsafe {
        let _ = eglGetError();
        if eglMakeCurrent(display, surface, surface, context) == egl::FALSE {
            return;
        }
        if width > 0 && height > 0 {
            gl::Viewport(0, 0, width, height);
            gl::ClearColor(0.12, 0.12, 0.18, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            eglSwapBuffers(display, surface);
        }
    }
}

use khronos_egl as egl;
use libloading::Library;
use nix::sys::memfd::{memfd_create, MFdFlags};
use nix::unistd::ftruncate;
use std::ffi::CString;
use libc;

const EGL_NONE: i32 = 0x3038;
const EGL_EXTENSIONS: i32 = 0x3055;
const EGL_LINUX_DMA_BUF_EXT: i32 = 0x3270;
const EGL_LINUX_DRM_FOURCC_EXT: i32 = 0x3271;
const EGL_DMA_BUF_PLANE0_FD_EXT: i32 = 0x3272;
const EGL_DMA_BUF_PLANE0_OFFSET_EXT: i32 = 0x3273;
const EGL_DMA_BUF_PLANE0_PITCH_EXT: i32 = 0x3274;
const EGL_IMAGE_PRESERVED_KHR: i32 = 0x30D2;
const EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT: i32 = 0x3444;
const EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT: i32 = 0x3445;

type EglCreateImageKHR = unsafe extern "C" fn(egl::EGLDisplay, egl::EGLContext, i32, *mut std::ffi::c_void, *const i32) -> *mut std::ffi::c_void;
type EglDestroyImageKHR = unsafe extern "C" fn(egl::EGLDisplay, *mut std::ffi::c_void) -> i32;

#[link(name = "EGL")]
#[allow(dead_code)]
extern "C" {
    fn eglGetDisplay(display_id: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn eglInitialize(display: *mut std::ffi::c_void, major: *mut i32, minor: *mut i32) -> u32;
    fn eglChooseConfig(display: *mut std::ffi::c_void, attrib_list: *const i32, configs: *mut *mut std::ffi::c_void, config_size: i32, num_config: *mut i32) -> u32;
    fn eglCreateWindowSurface(display: *mut std::ffi::c_void, config: *mut std::ffi::c_void, win: *mut std::ffi::c_void, attrib_list: *const i32) -> *mut std::ffi::c_void;
    fn eglCreateContext(display: *mut std::ffi::c_void, config: *mut std::ffi::c_void, share_context: *mut std::ffi::c_void, attrib_list: *const i32) -> *mut std::ffi::c_void;
    fn eglMakeCurrent(display: *mut std::ffi::c_void, draw: *mut std::ffi::c_void, read: *mut std::ffi::c_void, ctx: *mut std::ffi::c_void) -> u32;
    fn eglSwapBuffers(display: *mut std::ffi::c_void, surface: *mut std::ffi::c_void) -> u32;
    fn eglDestroySurface(display: *mut std::ffi::c_void, surface: *mut std::ffi::c_void) -> u32;
    fn eglDestroyContext(display: *mut std::ffi::c_void, ctx: *mut std::ffi::c_void) -> u32;
    fn eglTerminate(display: *mut std::ffi::c_void) -> u32;
    fn eglGetError() -> i32;
    fn eglGetConfigAttrib(display: *mut std::ffi::c_void, config: *mut std::ffi::c_void, attribute: i32, value: *mut i32) -> u32;
    fn eglGetProcAddress(procname: *const std::ffi::c_char) -> *mut std::ffi::c_void;
    fn eglSwapInterval(display: *mut std::ffi::c_void, interval: i32) -> u32;
    fn eglQueryString(display: *mut std::ffi::c_void, name: i32) -> *const std::ffi::c_char;
}

#[derive(Debug)]
pub struct ShmRegion {
    fd: OwnedFd,
    pub size: usize,
}

impl ShmRegion {
    pub fn fd(&self) -> i32 {
        self.fd.as_raw_fd()
    }
}

const AHB_PROTOCOL_MAGIC: u32 = 0x5041_4842;
const AHB_PROTOCOL_VERSION: u16 = 1;
const AHB_POOL_READY: u16 = 2;
const AHB_FRAME_SOURCE: u16 = 8;
const AHB_FRAME_CONSUMED: u16 = 9;
const AHB_SOURCE_LINEAR: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
struct AhbMessage {
    magic: u32,
    version: u16,
    message_type: u16,
    slot: u32,
    generation: u32,
    frame_id: u64,
    width: u32,
    height: u32,
    format: u32,
    stride: u32,
    fd_count: u32,
    flags: u32,
}

const _: () = assert!(std::mem::size_of::<AhbMessage>() == 48);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BrokerSendResult {
    Unsupported,
    Unavailable,
    Backpressured,
    Sent(u64),
}

#[derive(Debug)]
struct FrameSourceBroker {
    socket: Option<OwnedFd>,
    next_frame_id: u64,
    /// wire frame ID and originating compositor RenderFrame ID per AHB slot.
    in_flight: [Option<(u64, u64)>; 3],
    generation: u32,
    expected_size: (u32, u32),
    ready: bool,
}

impl FrameSourceBroker {
    fn new() -> Self {
        Self { socket: None, next_frame_id: 1, in_flight: [None; 3], generation: 0, expected_size: (0, 0), ready: false }
    }

    fn ensure_connected(&mut self) -> bool {
        if self.socket.is_some() { return true; }
        let Ok(socket) = nix::sys::socket::socket(
            AddressFamily::Unix, SockType::SeqPacket,
            SockFlag::SOCK_CLOEXEC | SockFlag::SOCK_NONBLOCK, None,
        ) else { return false; };
        let Ok(address) = UnixAddr::new_abstract(b"padputer-frame-source") else { return false; };
        if let Err(error) = connect(socket.as_raw_fd(), &address) {
            log::debug!("FRAME_SOURCE broker unavailable: {}", error);
            return false;
        }
        log::info!("FRAME_SOURCE broker connected; awaiting POOL_READY");
        self.socket = Some(socket);
        self.ready = false;
        true
    }

    fn disconnect(&mut self) {
        self.socket = None;
        self.in_flight = [None; 3];
        self.expected_size = (0, 0);
        self.ready = false;
    }

    fn poll_consumed(&mut self) -> Vec<u64> {
        let mut completed = Vec::new();
        let Some(socket) = self.socket.as_ref() else { return completed; };
        loop {
            let mut bytes = [0u8; std::mem::size_of::<AhbMessage>()];
            let received = {
                let mut iov = [IoSliceMut::new(&mut bytes)];
                recvmsg::<()>(socket.as_raw_fd(), &mut iov, None, MsgFlags::MSG_DONTWAIT)
                    .map(|message| message.bytes)
            };
            match received {
                Ok(received) if received == bytes.len() => {
                    let packet = unsafe { std::ptr::read_unaligned(bytes.as_ptr().cast::<AhbMessage>()) };
                    if packet.magic == AHB_PROTOCOL_MAGIC && packet.version == AHB_PROTOCOL_VERSION &&
                       packet.message_type == AHB_POOL_READY && packet.fd_count == 0 && packet.generation > 0 &&
                       packet.width > 0 && packet.height > 0 {
                        self.generation = packet.generation;
                        self.expected_size = (packet.width, packet.height);
                        self.next_frame_id = 1;
                        self.in_flight = [None; 3];
                        self.ready = true;
                        log::info!("FRAME_SOURCE POOL_READY generation={} size={}x{}", self.generation, packet.width, packet.height);
                        continue;
                    }
                    let valid = packet.magic == AHB_PROTOCOL_MAGIC &&
                        packet.version == AHB_PROTOCOL_VERSION &&
                        packet.message_type == AHB_FRAME_CONSUMED &&
                        packet.generation == self.generation && packet.fd_count == 0 &&
                        (packet.slot as usize) < self.in_flight.len() &&
                        self.in_flight[packet.slot as usize].map(|entry| entry.0) == Some(packet.frame_id);
                    if !valid {
                        log::error!("invalid FRAME_CONSUMED frame={} slot={} generation={} expected_generation={}", packet.frame_id, packet.slot, packet.generation, self.generation);
                        self.disconnect();
                        return completed;
                    }
                    let (_, compositor_frame) = self.in_flight[packet.slot as usize].take().unwrap();
                    completed.push(compositor_frame);
                    log::debug!("FRAME_CONSUMED frame={} slot={} compositor_frame={}", packet.frame_id, packet.slot, compositor_frame);
                }
                Ok(0) => {
                    log::info!("FRAME_SOURCE worker disconnected after draining releases");
                    self.disconnect();
                    return completed;
                }
                Ok(received) => {
                    log::error!("short AHB control packet: {} bytes", received);
                    self.disconnect();
                    return completed;
                }
                Err(nix::errno::Errno::EAGAIN) => return completed,
                Err(error) => {
                    log::warn!("AHB control receive failed: {}", error);
                    self.disconnect();
                    return completed;
                }
            }
        }
    }

    fn send(&mut self, slot: usize, compositor_frame: u64, item: &RenderItem) -> BrokerSendResult {
        let RenderItem::DmaBuf { fd, fourcc, modifier, offset, stride, width, height,
                                 is_cursor, .. } = item else { return BrokerSendResult::Unsupported; };
        if *modifier != 0 || *offset != 0 || *width <= 0 || *height <= 0 || *is_cursor {
            return BrokerSendResult::Unsupported;
        }
        if !self.ensure_connected() { return BrokerSendResult::Unavailable; }
        let _ = self.poll_consumed();
        if !self.ready { return BrokerSendResult::Backpressured; }
        if (*width as u32, *height as u32) != self.expected_size {
            log::warn!("FRAME_SOURCE reject mismatched desktop source {}x{} expected {}x{}", width, height, self.expected_size.0, self.expected_size.1);
            return BrokerSendResult::Unsupported;
        }
        let frame_id = self.next_frame_id;
        let wire_slot = ((frame_id - 1) % 3) as u32;
        if self.in_flight[wire_slot as usize].is_some() {
            return BrokerSendResult::Backpressured;
        }
        let packet = AhbMessage {
            magic: AHB_PROTOCOL_MAGIC, version: AHB_PROTOCOL_VERSION,
            message_type: AHB_FRAME_SOURCE, slot: wire_slot, generation: self.generation,
            frame_id, width: *width as u32, height: *height as u32,
            format: *fourcc, stride: *stride, fd_count: 1, flags: AHB_SOURCE_LINEAR,
        };
        let bytes = unsafe {
            std::slice::from_raw_parts((&packet as *const AhbMessage).cast::<u8>(), std::mem::size_of_val(&packet))
        };
        let iov = [IoSlice::new(bytes)];
        let fds = [fd.as_raw_fd()];
        let controls = [ControlMessage::ScmRights(&fds)];
        let socket = self.socket.as_ref().unwrap();
        match sendmsg::<()>(socket.as_raw_fd(), &iov, &controls, MsgFlags::empty(), None) {
            Ok(written) if written == bytes.len() => {
                self.next_frame_id += 1;
                self.in_flight[wire_slot as usize] = Some((frame_id, compositor_frame));
                log::info!("FRAME_SOURCE sent compositor DMA-BUF as frame={} wire_slot={} compositor_slot={} {}x{} fourcc=0x{:x} stride={}",
                           frame_id, wire_slot, slot, width, height, fourcc, stride);
                BrokerSendResult::Sent(frame_id)
            }
            result => {
                log::warn!("FRAME_SOURCE send failed: {:?}", result);
                self.disconnect();
                BrokerSendResult::Unavailable
            }
        }
    }
}

#[derive(Debug)]
struct PresentationSlotTracker {
    frames: [Option<u64>; 3],
    next: usize,
}

impl PresentationSlotTracker {
    fn new() -> Self { Self { frames: [None, None, None], next: 0 } }

    fn acquire(&mut self, frame_id: u64) -> Option<usize> {
        for offset in 0..self.frames.len() {
            let slot = (self.next + offset) % self.frames.len();
            if self.frames[slot].is_none() {
                self.frames[slot] = Some(frame_id);
                self.next = (slot + 1) % self.frames.len();
                return Some(slot);
            }
        }
        None
    }

    fn release(&mut self, slot: usize, frame_id: u64) -> bool {
        if self.frames.get(slot).copied().flatten() != Some(frame_id) { return false; }
        self.frames[slot] = None;
        true
    }
}

#[derive(Debug)]
pub struct AndroidSmithayState {
    pub native_window: Option<*mut ndk_sys::ANativeWindow>,
    pub egl_display: Option<egl::EGLDisplay>,
    pub egl_surface: Option<egl::EGLSurface>,
    pub egl_context: Option<egl::EGLContext>,
    pub egl_config: Option<egl::EGLConfig>,
    pub egl_lib: Option<Library>,
    pub gl_program: Option<u32>,
    pub gl_cursor_program: Option<u32>,
    pub surface_size: (i32, i32),
    /// Safe-area top inset: camera notch / status bar height in pixels.
    /// Touch Y coordinates must be offset by this before routing.
    pub y_offset: i32,
    pub physical_size_mm: (i32, i32),
    pub scroll_sensitivity: f32,
    pub refresh_rate: f32,
    pub shm_enabled: bool,
    pub shm_region: Option<ShmRegion>,
    pub frames_presented: u64,
    /// User-requested resolution override (set by SetResolution from Dashboard).
    /// When Some, SurfaceChanged uses this instead of native Android dimensions.
    pub requested_resolution: Option<(i32, i32)>,
    /// User-requested scale override (set by SetScale from Dashboard).
    /// When Some, SurfaceChanged re-applies this after surface recreation.
    pub requested_scale: Option<f32>,
    /// Current output scale for NDC coordinate conversion.
    /// Updated whenever scale changes (from Dashboard or surface recreation).
    pub current_scale: f32,
    /// Whether EGL_EXT_image_dma_buf_import is available on this device
    pub has_dmabuf_import: bool,
    /// Whether EGL_EXT_image_dma_buf_import_modifiers is available on this device
    pub has_dmabuf_import_modifiers: bool,
    /// EGL_EXT_image_dma_buf_import function pointers
    pub egl_create_image_khr: Option<EglCreateImageKHR>,
    pub egl_destroy_image_khr: Option<EglDestroyImageKHR>,
    /// GL_OES_EGL_image function pointer
    pub gl_egl_image_target_texture_2d_oes: Option<unsafe extern "C" fn(gl::types::GLenum, *mut std::ffi::c_void)>,
    /// Fragment shader for DMA-BUF textures (no BGR swap)
    pub gl_dmabuf_program: Option<u32>,
    pub gl_cursor_dmabuf_program: Option<u32>,
    presentation_slots: PresentationSlotTracker,
    source_broker: FrameSourceBroker,
    presentation_sender: crossbeam_channel::Sender<u64>,
}

impl AndroidSmithayState {
    pub fn new(presentation_sender: crossbeam_channel::Sender<u64>) -> Self {
        Self {
            native_window: None,
            egl_display: None,
            egl_surface: None,
            egl_context: None,
            egl_config: None,
            egl_lib: None,
            gl_program: None,
            gl_cursor_program: None,
            surface_size: (0, 0),
            y_offset: 0,
            physical_size_mm: (155, 87),
            scroll_sensitivity: 1.0,
            refresh_rate: 60.0,
            shm_enabled: false,
            shm_region: None,
            frames_presented: 0,
            requested_resolution: None,
            requested_scale: None,
            current_scale: 1.0,
            has_dmabuf_import: false,
            has_dmabuf_import_modifiers: false,
            egl_create_image_khr: None,
            egl_destroy_image_khr: None,
            gl_egl_image_target_texture_2d_oes: None,
            gl_dmabuf_program: None,
            gl_cursor_dmabuf_program: None,
            presentation_slots: PresentationSlotTracker::new(),
            source_broker: FrameSourceBroker::new(),
            presentation_sender,
        }
    }
}

// SAFETY: The state is owned by the compositor thread (single-thread access).
unsafe impl Send for AndroidSmithayState {}

pub(crate) struct CursorFrame {
    pub(crate) image_serial: u64,
    pub(crate) hotspot: (i32, i32),
    pub(crate) item: RenderItem,
}

impl std::fmt::Debug for CursorFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CursorFrame")
            .field("image_serial", &self.image_serial)
            .field("hotspot", &self.hotspot)
            .field("kind", &if matches!(&self.item, RenderItem::DmaBuf { .. }) { "dmabuf" } else { "shm" })
            .finish()
    }
}

pub(crate) struct RenderFrame {
    pub(crate) id: u64,
    pub(crate) items: Vec<RenderItem>,
    pub(crate) cursor: Option<CursorFrame>,
}

pub(crate) enum RenderItem {
    Shm {
        pixels: Vec<u8>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        scale: f32,
        is_cursor: bool,
    },
    DmaBuf {
        fd: OwnedFd,
        fourcc: u32,
        modifier: u64,
        offset: u32,
        stride: u32,
        width: i32,
        height: i32,
        x: i32,
        y: i32,
        scale: f32,
        is_cursor: bool,
    },
}

impl RenderItem {
    pub fn is_cursor(&self) -> bool {
        match self {
            RenderItem::Shm { is_cursor, .. } => *is_cursor,
            RenderItem::DmaBuf { is_cursor, .. } => *is_cursor,
        }
    }
}

/// Drain the render-item channel and composite everything.
/// The compositor thread owns the receiver; the Wayland thread sends
/// items via its sender (stored on `AndroidSeatRuntime`).
pub(crate) fn flush_deferred_composite(
    state: &mut AndroidSmithayState,
    rx: &crossbeam_channel::Receiver<RenderFrame>,
) {
    for compositor_frame in state.source_broker.poll_consumed() {
        let _ = state.presentation_sender.send(compositor_frame);
    }

    // Keep only the newest complete compositor frame. Merging queued Vecs
    // duplicates surfaces and destroys frame/damage boundaries.
    let mut latest: Option<RenderFrame> = None;
    while let Ok(frame) = rx.try_recv() {
        latest = Some(frame);
    }
    if let Some(frame) = latest {
        let Some(slot) = state.presentation_slots.acquire(frame.id) else {
            log::warn!("dropping compositor frame {}: all presentation slots busy", frame.id);
            return;
        };
        log::debug!("presenting deferred compositor frame {} in slot {} ({} items)", frame.id, slot, frame.items.len());
        if let Some(cursor) = frame.cursor.as_ref() {
            let (width, height, kind) = match &cursor.item {
                RenderItem::DmaBuf { width, height, .. } => (*width, *height, "dmabuf"),
                RenderItem::Shm { width, height, .. } => (*width, *height, "shm"),
            };
            log::info!("CURSOR_SOURCE_CAPTURED serial={} kind={} size={}x{} hotspot={},{}", cursor.image_serial, kind, width, height, cursor.hotspot.0, cursor.hotspot.1);
            use std::io::Write as _;
            if let Ok(mut trace) = std::fs::OpenOptions::new().create(true).append(true).open(
                "/data/user/0/io.padputer.waylandbridge/files/outer-cursor-source.log",
            ) {
                let _ = writeln!(trace, "CURSOR_SOURCE_CAPTURED serial={} kind={} size={}x{} hotspot={},{}", cursor.image_serial, kind, width, height, cursor.hotspot.0, cursor.hotspot.1);
            }
        }
        let broker_result = if frame.items.len() == 1 {
            state.source_broker.send(slot, frame.id, &frame.items[0])
        } else {
            BrokerSendResult::Unsupported
        };
        match broker_result {
            BrokerSendResult::Sent(worker_frame) => {
                // The app-domain Turnip worker owns composition into the Android
                // AHB slot. Never also mmap/upload this DMA-BUF through the legacy
                // direct presenter.
                log::debug!("compositor frame {} handed to AHB worker frame {}", frame.id, worker_frame);
            }
            BrokerSendResult::Backpressured => {
                // Preserve latest-frame scheduling without queueing an unbounded
                // number of duplicated DMA-BUF fds behind the three AHB slots.
                log::debug!("dropping compositor frame {}: AHB slots await FRAME_CONSUMED", frame.id);
            }
            BrokerSendResult::Unsupported | BrokerSendResult::Unavailable => {
                if state.native_window.is_some() {
                    composite_multi(state, &frame.items);
                    let _ = state.presentation_sender.send(frame.id);
                } else {
                    // AHB mode deliberately relinquishes the direct EGL window.
                    // Without a worker there is no presentation completion, so
                    // do not fabricate callbacks for a frame that was not shown.
                    log::debug!("dropping compositor frame {}: no active presentation backend", frame.id);
                }
            }
        }
        if !state.presentation_slots.release(slot, frame.id) {
            log::error!("presentation slot/frame mismatch: slot={} frame={}", slot, frame.id);
        }
    }
}

/// Lightweight suspend: destroy only the EGL surface + ANativeWindow.
/// Keep the EGL context, display, shaders, and textures alive so Wayland
/// clients continue rendering headless. On resume, only a new surface is created.
pub fn suspend_native_window_inner(state: &mut AndroidSmithayState) {
    if let Some(display) = state.egl_display {
        unsafe {
            let _ = eglMakeCurrent(display, egl::NO_SURFACE, egl::NO_SURFACE, egl::NO_CONTEXT);
            if let Some(surface) = state.egl_surface.take() {
                let _ = eglDestroySurface(display, surface);
            }
        }
        log::info!("EGL: Surface suspended (context kept alive)");
    }
    state.egl_surface = None;

    if let Some(ptr) = state.native_window.take() {
        unsafe {
            ndk_sys::ANativeWindow_release(ptr);
        }
        log::info!("Android backend: ANativeWindow released (context kept alive)");
    }
}

pub fn suspend_native_window(state: &mut AndroidSmithayState) {
    suspend_native_window_inner(state);
}

/// Full teardown: destroys EGL context, display, shaders, surface, and window.
/// Only used during final compositor shutdown (not for suspend/resume).
fn release_native_window_inner(state: &mut AndroidSmithayState) {
    if let Some(display) = state.egl_display {
        unsafe {
            let _ = eglMakeCurrent(display, egl::NO_SURFACE, egl::NO_SURFACE, egl::NO_CONTEXT);

            if let Some(prog) = state.gl_program {
                gl::DeleteProgram(prog);
                state.gl_program = None;
            }
            if let Some(prog) = state.gl_cursor_program {
                gl::DeleteProgram(prog);
                state.gl_cursor_program = None;
            }
            if let Some(prog) = state.gl_dmabuf_program {
                gl::DeleteProgram(prog);
                state.gl_dmabuf_program = None;
            }
            if let Some(prog) = state.gl_cursor_dmabuf_program {
                gl::DeleteProgram(prog);
                state.gl_cursor_dmabuf_program = None;
            }

            if let Some(surface) = state.egl_surface.take() {
                let _ = eglDestroySurface(display, surface);
            }
            if let Some(context) = state.egl_context.take() {
                let _ = eglDestroyContext(display, context);
            }
        }
        log::info!("EGL: Surface and Context destroyed");
    }

    state.egl_display = None;
    state.egl_surface = None;
    state.egl_context = None;
    state.egl_config = None;
    state.egl_lib = None;

    if let Some(ptr) = state.native_window.take() {
        unsafe {
            ndk_sys::ANativeWindow_release(ptr);
        }
        log::info!("Android backend: ANativeWindow released");
    }
}

/// Initialize or re-initialize EGL on the compositor thread.
/// Fast-path: if EGL context already exists, only create a new surface + make current.
/// Full init: first-time setup of display, config, context, shaders.
pub fn bind_native_window(state: &mut AndroidSmithayState, native_window_ptr: *mut ndk_sys::ANativeWindow) -> Result<(), String> {
    if native_window_ptr.is_null() {
        return Err("ANativeWindow pointer is null".to_string());
    }

    // SurfaceChanged can be emitted before the compositor command channel is
    // installed. ANativeWindow is authoritative once bound, so initialize the
    // render target dimensions here as well.
    let window_width = unsafe { ndk_sys::ANativeWindow_getWidth(native_window_ptr) };
    let window_height = unsafe { ndk_sys::ANativeWindow_getHeight(native_window_ptr) };
    if window_width > 0 && window_height > 0 {
        state.surface_size = (window_width, window_height);
        crate::android::command_channel::set_surface_size(window_width, window_height);
        log::info!(
            "Android backend: render target size initialized from ANativeWindow: {}x{}",
            window_width,
            window_height
        );
    }

    // ── Fast-path: context already alive, just create a new surface ──
    if let (Some(display), Some(context), Some(config)) = (state.egl_display, state.egl_context, state.egl_config) {
        // Destroy old surface if any (from a previous suspend)
        if state.egl_surface.is_some() {
            unsafe { let _ = eglDestroySurface(display, state.egl_surface.take().unwrap()); }
        }
        if let Some(old_window) = state.native_window.take() {
            unsafe { ndk_sys::ANativeWindow_release(old_window); }
        }

        unsafe { ndk_sys::ANativeWindow_acquire(native_window_ptr); }

        unsafe { ndk_sys::ANativeWindow_setBuffersGeometry(native_window_ptr, 0, 0, 2); }

        let mut attempts = 0;
        loop {
            let surface = unsafe { eglCreateWindowSurface(display, config, native_window_ptr as *mut _, std::ptr::null()) };
            if surface != egl::NO_SURFACE {
                state.egl_surface = Some(surface);
                state.native_window = Some(native_window_ptr);
                unsafe { let _ = eglMakeCurrent(display, surface, surface, context); }
                unsafe { let _ = eglMakeCurrent(display, egl::NO_SURFACE, egl::NO_SURFACE, egl::NO_CONTEXT); }
                log::info!("Android backend: ANativeWindow rebound (context reused)");
                return Ok(());
            }
            let err = unsafe { eglGetError() };
            if err == 0x3003 && attempts < 3 {
                attempts += 1;
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
            return Err(format!("EGL: CreateWindowSurface rebind failed 0x{:x}", err));
        }
    }

    // ── Full init path (first-time setup) ──
    release_native_window_inner(state);

    unsafe {
        ndk_sys::ANativeWindow_acquire(native_window_ptr);
    }

    let egl_lib = unsafe { Library::new("libEGL.so") }.map_err(|e| format!("Failed to load libEGL.so: {e}"))?;

    let display = unsafe { eglGetDisplay(std::ptr::null_mut()) };
    if display == egl::NO_DISPLAY {
        return Err("EGL: GetDisplay failed".to_string());
    }

    let mut major = 0;
    let mut minor = 0;
    if unsafe { eglInitialize(display, &mut major, &mut minor) } == egl::FALSE {
        return Err("EGL: Initialize failed".to_string());
    }
    log::info!("EGL: Initialized version {}.{}", major, minor);

    let egl_extensions = unsafe {
        let ptr = eglQueryString(display, EGL_EXTENSIONS);
        if ptr.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(ptr).to_string_lossy().to_string()
        }
    };
    let has_dmabuf = egl_extensions.contains("EGL_EXT_image_dma_buf_import");
    let has_dmabuf_mods = egl_extensions.contains("EGL_EXT_image_dma_buf_import_modifiers");
    let snippet = if egl_extensions.len() > 200 { &egl_extensions[..200] } else { &egl_extensions[..] };
    log::info!("EGL has_dmabuf_import={} has_dmabuf_modifiers={} extensions={}",
               has_dmabuf, has_dmabuf_mods, snippet);
    state.has_dmabuf_import = has_dmabuf;
    state.has_dmabuf_import_modifiers = has_dmabuf_mods;

    // Load EGL extension function pointers for DMA-BUF import
    if has_dmabuf {
        unsafe {
            let create_name = std::ffi::CString::new("eglCreateImageKHR").unwrap();
            let create_ptr = eglGetProcAddress(create_name.as_ptr());
            if !create_ptr.is_null() {
                state.egl_create_image_khr = Some(std::mem::transmute::<_, EglCreateImageKHR>(create_ptr));
            }
            let destroy_name = std::ffi::CString::new("eglDestroyImageKHR").unwrap();
            let destroy_ptr = eglGetProcAddress(destroy_name.as_ptr());
            if !destroy_ptr.is_null() {
                state.egl_destroy_image_khr = Some(std::mem::transmute::<_, EglDestroyImageKHR>(destroy_ptr));
            }
        }
    }

    // Load GL_OES_EGL_image extension
    {
        let name = std::ffi::CString::new("glEGLImageTargetTexture2DOES").unwrap();
        let ptr = unsafe { eglGetProcAddress(name.as_ptr()) };
        if !ptr.is_null() {
            state.gl_egl_image_target_texture_2d_oes = Some(
                unsafe { std::mem::transmute::<_, unsafe extern "C" fn(gl::types::GLenum, *mut std::ffi::c_void)>(ptr) }
            );
        }
    }

    unsafe {
        ndk_sys::ANativeWindow_setBuffersGeometry(native_window_ptr, 0, 0, 2);
    }

    let mut config: egl::EGLConfig = std::ptr::null_mut();
    let mut num_config = 0;
    let config_attempts = [
        [
            egl::RED_SIZE, 8, egl::GREEN_SIZE, 8, egl::BLUE_SIZE, 8, egl::ALPHA_SIZE, 0,
            egl::SURFACE_TYPE, egl::WINDOW_BIT, egl::NONE,
        ],
        [
            egl::RED_SIZE, 8, egl::GREEN_SIZE, 8, egl::BLUE_SIZE, 8, egl::ALPHA_SIZE, 8,
            egl::SURFACE_TYPE, egl::WINDOW_BIT, egl::NONE,
        ],
        [
            egl::RED_SIZE, 5, egl::GREEN_SIZE, 6, egl::BLUE_SIZE, 5, egl::ALPHA_SIZE, 0,
            egl::SURFACE_TYPE, egl::WINDOW_BIT, egl::NONE,
        ],
    ];

    let mut found_config = false;
    for (i, attempt) in config_attempts.iter().enumerate() {
        if unsafe { eglChooseConfig(display, attempt.as_ptr(), &mut config, 1, &mut num_config) } != egl::FALSE && num_config > 0 {
            log::info!("EGL: Successfully selected configuration priority {}", i + 1);
            found_config = true;
            break;
        }
    }

    if !found_config {
        return Err("EGL: ChooseConfig failed".to_string());
    }

    let mut surface = egl::NO_SURFACE;
    let mut attempts = 0;
    const MAX_ATTEMPTS: i32 = 3;

    while attempts < MAX_ATTEMPTS {
        surface = unsafe { eglCreateWindowSurface(display, config, native_window_ptr as *mut _, std::ptr::null()) };
        if surface != egl::NO_SURFACE {
            break;
        }
        let err = unsafe { eglGetError() };
        if err == 0x3003 {
            log::warn!("EGL: CreateWindowSurface busy (0x3003), retrying... (attempt {}/{})", attempts + 1, MAX_ATTEMPTS);
            std::thread::sleep(std::time::Duration::from_millis(100));
            attempts += 1;
        } else {
            return Err(format!("EGL: CreateWindowSurface failed with fatal error 0x{:x}", err));
        }
    }

    if surface == egl::NO_SURFACE {
        return Err(format!("EGL: CreateWindowSurface failed after {} attempts", MAX_ATTEMPTS));
    }

    let ctx_attribs = [egl::CONTEXT_CLIENT_VERSION, 2, egl::NONE];
    let context = unsafe { eglCreateContext(display, config, egl::NO_CONTEXT, ctx_attribs.as_ptr()) };
    if context == egl::NO_CONTEXT {
        return Err("EGL: CreateContext failed".to_string());
    }

    if unsafe { eglMakeCurrent(display, surface, surface, context) } == egl::FALSE {
        return Err("EGL: MakeCurrent failed".to_string());
    }

    gl::load_with(|s| {
        let name = std::ffi::CString::new(s).expect("Failed to create CString for eglGetProcAddress");
        unsafe { eglGetProcAddress(name.as_ptr() as *const _) as *const _ }
    });

    if state.gl_program.is_none() {
        let vs_src = "attribute vec2 aPos; attribute vec2 aTex; varying vec2 vTex; void main(){ vTex=aTex; gl_Position=vec4(aPos,0.0,1.0); }";
        let fs_src = "precision mediump float; varying vec2 vTex; uniform sampler2D uTex; void main(){ vec4 c=texture2D(uTex,vTex); gl_FragColor=vec4(c.b,c.g,c.r,1.0); }";
        let fs_cursor_src = "precision mediump float; varying vec2 vTex; uniform sampler2D uTex; void main(){ vec4 c=texture2D(uTex,vTex); gl_FragColor=vec4(c.b,c.g,c.r,c.a); }";

        // DMA-BUF textures are already in R,G,B,A order from EGL import (no BGR swap)
        let fs_dmabuf_src = "precision mediump float; varying vec2 vTex; uniform sampler2D uTex; void main(){ vec4 c=texture2D(uTex,vTex); gl_FragColor=vec4(c.r,c.g,c.b,1.0); }";
        let fs_cursor_dmabuf_src = "precision mediump float; varying vec2 vTex; uniform sampler2D uTex; void main(){ vec4 c=texture2D(uTex,vTex); gl_FragColor=vec4(c.r,c.g,c.b,c.a); }";

        unsafe {
            let vs = gl::CreateShader(gl::VERTEX_SHADER);
            let c_vs = std::ffi::CString::new(vs_src).unwrap();
            gl::ShaderSource(vs, 1, &c_vs.as_ptr(), std::ptr::null());
            gl::CompileShader(vs);

            let fs = gl::CreateShader(gl::FRAGMENT_SHADER);
            let c_fs = std::ffi::CString::new(fs_src).unwrap();
            gl::ShaderSource(fs, 1, &c_fs.as_ptr(), std::ptr::null());
            gl::CompileShader(fs);
            let program = gl::CreateProgram();
            gl::AttachShader(program, vs);
            gl::AttachShader(program, fs);
            gl::BindAttribLocation(program, 0, std::ffi::CString::new("aPos").unwrap().as_ptr());
            gl::BindAttribLocation(program, 1, std::ffi::CString::new("aTex").unwrap().as_ptr());
            gl::LinkProgram(program);
            state.gl_program = Some(program as u32);
            gl::DeleteShader(fs);

            let fs_cursor = gl::CreateShader(gl::FRAGMENT_SHADER);
            let c_fs_cursor = std::ffi::CString::new(fs_cursor_src).unwrap();
            gl::ShaderSource(fs_cursor, 1, &c_fs_cursor.as_ptr(), std::ptr::null());
            gl::CompileShader(fs_cursor);
            let cursor_program = gl::CreateProgram();
            gl::AttachShader(cursor_program, vs);
            gl::AttachShader(cursor_program, fs_cursor);
            gl::BindAttribLocation(cursor_program, 0, std::ffi::CString::new("aPos").unwrap().as_ptr());
            gl::BindAttribLocation(cursor_program, 1, std::ffi::CString::new("aTex").unwrap().as_ptr());
            gl::LinkProgram(cursor_program);
            state.gl_cursor_program = Some(cursor_program as u32);
            gl::DeleteShader(fs_cursor);

            // DMA-BUF shaders (no BGR swap)
            let fs_dmabuf = gl::CreateShader(gl::FRAGMENT_SHADER);
            let c_fs_dmabuf = std::ffi::CString::new(fs_dmabuf_src).unwrap();
            gl::ShaderSource(fs_dmabuf, 1, &c_fs_dmabuf.as_ptr(), std::ptr::null());
            gl::CompileShader(fs_dmabuf);
            let dmabuf_program = gl::CreateProgram();
            gl::AttachShader(dmabuf_program, vs);
            gl::AttachShader(dmabuf_program, fs_dmabuf);
            gl::BindAttribLocation(dmabuf_program, 0, std::ffi::CString::new("aPos").unwrap().as_ptr());
            gl::BindAttribLocation(dmabuf_program, 1, std::ffi::CString::new("aTex").unwrap().as_ptr());
            gl::LinkProgram(dmabuf_program);
            state.gl_dmabuf_program = Some(dmabuf_program as u32);
            gl::DeleteShader(fs_dmabuf);

            let fs_cursor_dmabuf = gl::CreateShader(gl::FRAGMENT_SHADER);
            let c_fs_cursor_dmabuf = std::ffi::CString::new(fs_cursor_dmabuf_src).unwrap();
            gl::ShaderSource(fs_cursor_dmabuf, 1, &c_fs_cursor_dmabuf.as_ptr(), std::ptr::null());
            gl::CompileShader(fs_cursor_dmabuf);
            let cursor_dmabuf_program = gl::CreateProgram();
            gl::AttachShader(cursor_dmabuf_program, vs);
            gl::AttachShader(cursor_dmabuf_program, fs_cursor_dmabuf);
            gl::BindAttribLocation(cursor_dmabuf_program, 0, std::ffi::CString::new("aPos").unwrap().as_ptr());
            gl::BindAttribLocation(cursor_dmabuf_program, 1, std::ffi::CString::new("aTex").unwrap().as_ptr());
            gl::LinkProgram(cursor_dmabuf_program);
            state.gl_cursor_dmabuf_program = Some(cursor_dmabuf_program as u32);
            gl::DeleteShader(fs_cursor_dmabuf);

            gl::DeleteShader(vs);
        }
    }

    unsafe {
        let _ = eglMakeCurrent(display, egl::NO_SURFACE, egl::NO_SURFACE, egl::NO_CONTEXT);
    }

    state.native_window = Some(native_window_ptr);
    state.egl_display = Some(display);
    state.egl_surface = Some(surface);
    state.egl_context = Some(context);
    state.egl_config = Some(config);
    state.egl_lib = Some(egl_lib);
    log::info!("Android backend: ANativeWindow bound and EGL initialized");
    Ok(())
}

pub fn release_native_window(state: &mut AndroidSmithayState) {
    release_native_window_inner(state);
}

pub fn enable_shm_on_compositor(state: &mut AndroidSmithayState) {
    state.shm_enabled = true;
    if state.shm_region.is_none() {
        match create_memfd_region(4 * 1024 * 1024) {
            Ok(region) => {
                log::info!("Android backend: allocated SHM memfd fd={} size={}", region.fd(), region.size);
                state.shm_region = Some(region);
            }
            Err(err) => {
                log::error!("Android backend: failed to allocate SHM memfd: {}", err);
            }
        }
    }
}

pub fn create_memfd_region(size: usize) -> Result<ShmRegion, String> {
    if size == 0 {
        return Err("memfd size must be > 0".to_string());
    }

    let name = CString::new("winland-shm").map_err(|e| e.to_string())?;
    let fd = memfd_create(name.as_c_str(), MFdFlags::MFD_CLOEXEC)
        .map_err(|e| format!("memfd_create failed: {e}"))?;

    ftruncate(&fd, size as i64).map_err(|e| format!("ftruncate failed: {e}"))?;

    Ok(ShmRegion { fd, size })
}

#[cfg(feature = "smithay_android")]
pub mod smithay_globals {
    use smithay::reexports::wayland_server::protocol::wl_shm::WlShm;
    use smithay::reexports::wayland_server::{
        protocol::wl_shm_pool::WlShmPool, DisplayHandle, Dispatch, GlobalDispatch,
    };
    use smithay::wayland::buffer::BufferHandler;
    use smithay::wayland::shm::{ShmBufferUserData, ShmHandler, ShmPoolUserData, ShmState};

    // Helper to create wl_shm global backed by Smithay's memfd-based shm implementation.
    pub fn create_shm_global<D>(display: &DisplayHandle) -> ShmState
    where
        D: GlobalDispatch<WlShm, smithay::wayland::GlobalData>
            + Dispatch<WlShm, smithay::wayland::GlobalData>
            + Dispatch<WlShmPool, ShmPoolUserData>
            + Dispatch<smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer, ShmBufferUserData>
            + BufferHandler
            + ShmHandler
            + 'static,
    {
        ShmState::new::<D>(display, [])
    }
}