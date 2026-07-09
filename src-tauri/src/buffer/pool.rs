//! Frame memory pool — recycles frame data buffers.
//!
//! The capture backend acquires buffers via [`FramePool::acquire`] and fills
//! them with pixel data. The resulting `Vec<u8>` is wrapped in `Arc<Vec<u8>>`
//! for sharing, so pooled buffers cannot be automatically returned on drop.
//!
//! Instead, the pool serves as a fallback allocator: it hands out pre-sized
//! buffers when available and allocates fresh ones when empty. The ring buffer
//! eviction path can optionally call [`FramePool::release`] to recycle.
//!
//! Currently the pool is **not connected** to the live pipeline — frames flow
//! through direct `Vec::with_capacity` allocation in the capture backend.
//! This module exists for future optimization work.

use std::sync::Mutex;

pub struct FramePool {
    _pool: Mutex<Vec<Vec<u8>>>,
    _buffer_size: usize,
}

impl FramePool {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            _pool: Mutex::new(Vec::new()),
            _buffer_size: buffer_size,
        }
    }

    #[allow(dead_code)]
    pub fn acquire(&self) -> Vec<u8> {
        Vec::with_capacity(self._buffer_size)
    }

    #[allow(dead_code)]
    pub fn release(&self, buf: Vec<u8>) {
        drop(buf);
    }

    #[allow(dead_code)]
    pub fn resize(&self, _new_size: usize) {}
}
