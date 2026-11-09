//! A stand-in for the macOS headless GL surface so the same test and example
//! targets build on other hosts. Creating one fails, and callers skip.

use crate::surface::{GfxError, Surface, DEFAULT_PANEL_H, DEFAULT_PANEL_W};

pub struct HeadlessSurface;

impl HeadlessSurface {
    pub fn new() -> Result<Self, GfxError> {
        Err(GfxError::Context(
            "headless rendering is only available on macOS".into(),
        ))
    }
}

impl Surface for HeadlessSurface {
    fn make_current(&mut self) -> Result<(), GfxError> {
        Err(GfxError::Context(
            "headless rendering is only available on macOS".into(),
        ))
    }

    fn window_size(&self) -> (u32, u32) {
        (DEFAULT_PANEL_W, DEFAULT_PANEL_H)
    }

    fn swap(&mut self) -> Result<(), GfxError> {
        Ok(())
    }

    fn proc_address(&self, _name: &str) -> *const std::ffi::c_void {
        std::ptr::null()
    }
}
