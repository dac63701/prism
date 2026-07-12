use std::collections::VecDeque;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};
use wasapi::{initialize_mta, AudioClient, Direction, SampleType, StreamMode, WaveFormat};

use super::analyzer::AudioAnalyzer;
use super::RustAudioEngine;

pub(super) fn spawn_capture(app: AppHandle, pid: u32, generation: u64, sensitivity: f32) {
    let _ = thread::Builder::new()
        .name("prism-rust-audio".into())
        .spawn(move || {
            let result = capture_loop(&app, pid, generation, sensitivity);
            if let Err(error) = result {
                eprintln!("[rust-audio] capture ended: {error}");
            }
            app.state::<RustAudioEngine>()
                .mark_capture_stopped(generation);
        });
}

fn capture_loop(
    app: &AppHandle,
    pid: u32,
    generation: u64,
    sensitivity: f32,
) -> Result<(), String> {
    initialize_mta()
        .ok()
        .map_err(|error| format!("COM initialization failed: {error}"))?;

    // Process loopback is Windows-provided capture. It does not install hooks,
    // inspect process memory, or interact with the game's renderer.
    let format = WaveFormat::new(32, 32, &SampleType::Float, 16_000, 1, None);
    let mut client = AudioClient::new_application_loopback_client(pid, true)
        .map_err(|error| format!("process loopback activation failed: {error}"))?;
    let mode = StreamMode::PollingShared {
        autoconvert: true,
        buffer_duration_hns: 0,
    };
    client
        .initialize_client(&format, &Direction::Capture, &mode)
        .map_err(|error| format!("process loopback initialization failed: {error}"))?;
    let capture = client
        .get_audiocaptureclient()
        .map_err(|error| format!("process loopback capture client failed: {error}"))?;
    client
        .start_stream()
        .map_err(|error| format!("process loopback start failed: {error}"))?;

    let mut analyzer = AudioAnalyzer::new(sensitivity);
    let mut bytes = VecDeque::new();
    while app.state::<RustAudioEngine>().is_current(pid, generation) {
        let packet = capture
            .get_next_packet_size()
            .map_err(|error| format!("process loopback packet read failed: {error}"))?
            .unwrap_or_default();
        if packet > 0 {
            capture
                .read_from_device_to_deque(&mut bytes)
                .map_err(|error| format!("process loopback sample read failed: {error}"))?;
            let mut samples = Vec::with_capacity(bytes.len() / 4);
            while bytes.len() >= 4 {
                let chunk = [
                    bytes.pop_front().unwrap_or_default(),
                    bytes.pop_front().unwrap_or_default(),
                    bytes.pop_front().unwrap_or_default(),
                    bytes.pop_front().unwrap_or_default(),
                ];
                samples.push(f32::from_le_bytes(chunk));
            }
            for moment in analyzer.push_samples(&samples) {
                crate::games::trigger::trigger_auto_clip(app, moment);
            }
        }
        thread::sleep(Duration::from_millis(10));
    }

    let _ = client.stop_stream();
    Ok(())
}
