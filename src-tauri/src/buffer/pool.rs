//! Frame memory pool — pre-allocates frame data buffers for reuse.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

/// Fixed-size pool of pre-allocated frame data buffers.
pub struct FramePool {
    pool: Mutex<Vec<Vec<u8>>>,
    buffer_size: usize,
    created: AtomicU32,
    reused: AtomicU32,
}

impl FramePool {
    /// Create a pool that allocates buffers of `buffer_size` bytes.
    pub fn new(buffer_size: usize, prealloc: usize) -> Self {
        let mut frames = Vec::with_capacity(prealloc);
        for _ in 0..prealloc {
            frames.push(vec![0u8; buffer_size]);
        }
        Self {
            pool: Mutex::new(frames),
            buffer_size,
            created: AtomicU32::new(prealloc as u32),
            reused: AtomicU32::new(0),
        }
    }

    /// Acquire a buffer from the pool, or allocate one if empty.
    pub fn acquire(&self) -> Vec<u8> {
        let mut guard = self.pool.lock().unwrap();
        if let Some(buf) = guard.pop() {
            self.reused.fetch_add(1, Ordering::Relaxed);
            buf
        } else {
            drop(guard);
            self.created.fetch_add(1, Ordering::Relaxed);
            vec![0u8; self.buffer_size]
        }
    }

    /// Return a buffer to the pool for reuse.
    pub fn release(&self, buf: Vec<u8>) {
        let mut guard = self.pool.lock().unwrap();
        guard.push(buf);
    }

    /// Pool statistics: (total_created, total_reused).
    pub fn stats(&self) -> (u32, u32) {
        (
            self.created.load(Ordering::Relaxed),
            self.reused.load(Ordering::Relaxed),
        )
    }
}
