//! Platform-specific implementation (stub)
//! Shared audio detection terminology.
//!
//! Platform capture is implemented by game modules. The Rust implementation
//! uses Windows WASAPI application loopback, which reads only final audio output
//! and never touches game process memory.
