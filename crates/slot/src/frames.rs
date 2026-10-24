use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

pub struct Frames {
    inner: Mutex<Inner>,
    size: usize,
    taken: AtomicU64,
}

struct Inner {
    ready: Option<Frame>,
    spare: Vec<Vec<u8>>,
    allocated: usize,
}

struct Frame {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl Frames {
    pub fn new(size: usize) -> Arc<Self> {
        Arc::new(Frames {
            inner: Mutex::new(Inner {
                ready: None,
                spare: Vec::new(),
                allocated: 0,
            }),
            size,
            taken: AtomicU64::new(0),
        })
    }

    pub fn take_write(&self) -> Vec<u8> {
        let mut i = self.lock();
        match i.spare.pop() {
            Some(buf) => buf,
            None => {
                i.allocated += 1;
                Vec::with_capacity(self.size)
            }
        }
    }

    pub fn publish(&self, pixels: Vec<u8>, (width, height): (u32, u32)) {
        let frame = Frame {
            pixels,
            width,
            height,
        };
        let mut i = self.lock();
        if let Some(dropped) = i.ready.replace(frame) {
            i.spare.push(dropped.pixels);
        }
    }

    pub fn latest(self: &Arc<Self>) -> Option<FrameRef> {
        let frame = self.lock().ready.take()?;
        self.taken.fetch_add(1, Ordering::Relaxed);
        Some(FrameRef {
            frames: self.clone(),
            pixels: frame.pixels,
            width: frame.width,
            height: frame.height,
        })
    }

    pub fn is_ready(&self) -> bool {
        self.lock().ready.is_some()
    }

    pub fn taken(&self) -> u64 {
        self.taken.load(Ordering::Relaxed)
    }

    pub fn allocated(&self) -> usize {
        self.lock().allocated
    }

    fn recycle(&self, buf: Vec<u8>) {
        self.lock().spare.push(buf);
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }
}

pub struct FrameRef {
    frames: Arc<Frames>,
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl FrameRef {
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Deref for FrameRef {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.pixels
    }
}

impl Drop for FrameRef {
    fn drop(&mut self) {
        self.frames.recycle(std::mem::take(&mut self.pixels));
    }
}
