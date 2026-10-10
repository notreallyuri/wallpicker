//! Runtime-loaded Wayland EGL window; owns the native handle and its surface.

use libloading::Library;
use std::ffi::c_void;
use std::ptr::NonNull;
use wayland_client::{Proxy, protocol::wl_surface::WlSurface};

type Create = unsafe extern "C" fn(*mut c_void, i32, i32) -> *mut c_void;
type Destroy = unsafe extern "C" fn(*mut c_void);

pub(super) struct Window {
    ptr: NonNull<c_void>,
    destroy: Destroy,
    _library: Library,
    // Keep the Wayland proxy alive until the native window is destroyed.
    _surface: WlSurface,
}

impl Window {
    pub(super) fn new(surface: &WlSurface, size: (u32, u32)) -> Result<Self, String> {
        // SAFETY: symbols have the signatures from wayland-egl.h. The library
        // and surface outlive the window; this runs only on the renderer thread.
        unsafe {
            let library = Library::new("libwayland-egl.so.1").map_err(|e| e.to_string())?;
            let create: Create = *library
                .get(b"wl_egl_window_create\0")
                .map_err(|e| e.to_string())?;
            let destroy: Destroy = *library
                .get(b"wl_egl_window_destroy\0")
                .map_err(|e| e.to_string())?;
            let ptr = NonNull::new(create(
                surface.id().as_ptr().cast(),
                size.0 as i32,
                size.1 as i32,
            ))
            .ok_or("could not create Wayland EGL window")?;
            Ok(Self {
                ptr,
                destroy,
                _library: library,
                _surface: surface.clone(),
            })
        }
    }

    pub(super) fn ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // SAFETY: the handle is owned, valid, and destroyed exactly once.
        unsafe {
            (self.destroy)(self.ptr.as_ptr());
        }
    }
}
