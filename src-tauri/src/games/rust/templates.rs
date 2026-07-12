use crate::games::moment::MomentType;

pub(super) struct SoundProfile {
    pub name: &'static str,
    pub event: MomentType,
    pub bands: [f32; 8],
    pub minimum_similarity: f32,
}

// Conservative starter profiles use normalized spectral energy bands. They are
// deliberately only consulted after a loud onset; per-game sensitivity controls
// the match threshold. Additional recordings can be added without changing the
// capture path or anti-cheat posture.
pub(super) const PROFILES: &[SoundProfile] = &[
    SoundProfile {
        name: "headshot ding",
        event: MomentType::Headshot,
        bands: [0.01, 0.02, 0.05, 0.10, 0.17, 0.31, 0.24, 0.10],
        minimum_similarity: 0.89,
    },
    SoundProfile {
        name: "AK-style gunshot",
        event: MomentType::Combat,
        bands: [0.33, 0.25, 0.17, 0.10, 0.07, 0.04, 0.03, 0.01],
        minimum_similarity: 0.83,
    },
    SoundProfile {
        name: "bolt-action gunshot",
        event: MomentType::Combat,
        bands: [0.05, 0.08, 0.12, 0.15, 0.19, 0.21, 0.14, 0.06],
        minimum_similarity: 0.84,
    },
    SoundProfile {
        name: "shotgun blast",
        event: MomentType::Combat,
        bands: [0.38, 0.29, 0.16, 0.08, 0.05, 0.02, 0.01, 0.01],
        minimum_similarity: 0.84,
    },
    SoundProfile {
        name: "rocket or C4 explosion",
        event: MomentType::Explosion,
        bands: [0.49, 0.21, 0.11, 0.07, 0.05, 0.03, 0.02, 0.02],
        minimum_similarity: 0.86,
    },
];
