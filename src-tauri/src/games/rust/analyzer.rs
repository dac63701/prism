use std::collections::VecDeque;
use std::time::{Duration, Instant};

use realfft::RealFftPlanner;

use crate::games::moment::{GameMoment, MomentType};

use super::templates::PROFILES;

const SAMPLE_RATE: f32 = 16_000.0;
const WINDOW_SIZE: usize = 2_048;
const HOP_SIZE: usize = 512;

pub(super) struct AudioAnalyzer {
    samples: Vec<f32>,
    noise_floor: f32,
    sensitivity: f32,
    onsets: VecDeque<Instant>,
    last_combat: Option<Instant>,
}

impl AudioAnalyzer {
    pub(super) fn new(sensitivity: f32) -> Self {
        Self {
            samples: Vec::with_capacity(WINDOW_SIZE * 2),
            noise_floor: 0.003,
            sensitivity,
            onsets: VecDeque::new(),
            last_combat: None,
        }
    }

    pub(super) fn push_samples(&mut self, input: &[f32]) -> Vec<GameMoment> {
        self.samples.extend_from_slice(input);
        let mut moments = Vec::new();
        while self.samples.len() >= WINDOW_SIZE {
            let window = &self.samples[..WINDOW_SIZE];
            let energy = rms(window);
            let minimum_energy = 0.02 + (1.0 - self.sensitivity) * 0.08;
            let onset = energy > minimum_energy.max(self.noise_floor * 3.0);
            if onset {
                let now = Instant::now();
                self.onsets.push_back(now);
                while self
                    .onsets
                    .front()
                    .is_some_and(|time| now.duration_since(*time) > Duration::from_secs(3))
                {
                    self.onsets.pop_front();
                }

                if let Some((event, description)) = classify(window, self.sensitivity) {
                    moments.push(
                        GameMoment::new(event, "Rust".into())
                            .with_description(format!("Rust audio: {description}")),
                    );
                } else if self.onsets.len() >= 5
                    && self
                        .last_combat
                        .is_none_or(|last| now.duration_since(last) > Duration::from_secs(5))
                {
                    self.last_combat = Some(now);
                    moments.push(
                        GameMoment::new(MomentType::Combat, "Rust".into())
                            .with_description("Rust sustained gunfire".into()),
                    );
                }
            } else {
                self.noise_floor = self.noise_floor * 0.98 + energy * 0.02;
            }
            self.samples.drain(..HOP_SIZE);
        }
        moments
    }
}

fn classify(window: &[f32], sensitivity: f32) -> Option<(MomentType, String)> {
    let features = spectral_bands(window)?;
    let mut best: Option<(MomentType, &str, f32)> = None;
    for profile in PROFILES {
        let score = cosine_similarity(&features, &profile.bands);
        let threshold = (profile.minimum_similarity - sensitivity * 0.08).clamp(0.75, 0.98);
        if score >= threshold && best.is_none_or(|(_, _, current)| score > current) {
            best = Some((profile.event, profile.name, score));
        }
    }
    best.map(|(event, name, score)| (event, format!("{name} ({:.0}% match)", score * 100.0)))
}

fn spectral_bands(window: &[f32]) -> Option<[f32; 8]> {
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(WINDOW_SIZE);
    let mut input = window
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let hann = 0.5
                * (1.0
                    - (2.0 * std::f32::consts::PI * index as f32 / (WINDOW_SIZE - 1) as f32).cos());
            sample * hann
        })
        .collect::<Vec<_>>();
    let mut output = fft.make_output_vec();
    fft.process(&mut input, &mut output).ok()?;

    let ranges = [
        (200.0, 400.0),
        (400.0, 800.0),
        (800.0, 1200.0),
        (1200.0, 1800.0),
        (1800.0, 2500.0),
        (2500.0, 3500.0),
        (3500.0, 5000.0),
        (5000.0, 8000.0),
    ];
    let bin_hz = SAMPLE_RATE / WINDOW_SIZE as f32;
    let mut bands = [0.0; 8];
    for (index, (low, high)) in ranges.into_iter().enumerate() {
        let start = (low / bin_hz).floor() as usize;
        let end = ((high / bin_hz).ceil() as usize).min(output.len());
        bands[index] = output[start..end]
            .iter()
            .map(|value| value.norm_sqr())
            .sum::<f32>();
    }
    let norm = bands.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return None;
    }
    for value in &mut bands {
        *value /= norm;
    }
    Some(bands)
}

fn cosine_similarity(left: &[f32; 8], right: &[f32; 8]) -> f32 {
    let dot = left.iter().zip(right).map(|(a, b)| a * b).sum::<f32>();
    let left_norm = left.iter().map(|value| value * value).sum::<f32>().sqrt();
    let right_norm = right.iter().map(|value| value * value).sum::<f32>().sqrt();
    if left_norm <= f32::EPSILON || right_norm <= f32::EPSILON {
        0.0
    } else {
        dot / (left_norm * right_norm)
    }
}

fn rms(samples: &[f32]) -> f32 {
    (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32).sqrt()
}

#[cfg(test)]
mod tests {
    use super::cosine_similarity;

    #[test]
    fn cosine_similarity_is_one_for_equal_vectors() {
        let vector = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        assert!((cosine_similarity(&vector, &vector) - 1.0).abs() < 0.0001);
    }
}
