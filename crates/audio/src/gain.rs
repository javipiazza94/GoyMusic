//! Software gain: per-track ReplayGain (applied while decoding) and the volume curve
//! (applied by the sink, so volume changes are heard at once, not after the buffer drains).

use std::path::Path;

use lofty::prelude::*;
use lofty::tag::ItemKey;

/// ReplayGain values from a file's tags.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReplayGain {
    pub track_gain_db: Option<f32>,
    pub track_peak: Option<f32>,
}

impl ReplayGain {
    /// Reads the track gain and peak; missing or unreadable tags give `ReplayGain::default()`.
    pub fn read(path: &Path) -> Self {
        let Ok(file) = lofty::read_from_path(path) else {
            return Self::default();
        };
        let Some(tag) = file.primary_tag().or_else(|| file.first_tag()) else {
            return Self::default();
        };
        Self {
            track_gain_db: tag.get_string(&ItemKey::ReplayGainTrackGain).and_then(parse_number),
            track_peak: tag.get_string(&ItemKey::ReplayGainTrackPeak).and_then(parse_number),
        }
    }

    /// Linear factor for the track; never pushes the declared peak above full scale.
    /// No gain tag means 1.0.
    pub fn factor(&self) -> f32 {
        let Some(db) = self.track_gain_db else {
            return 1.0;
        };
        let factor = 10f32.powf(db / 20.0);
        match self.track_peak {
            Some(peak) if peak > 0.0 => factor.min(1.0 / peak),
            _ => factor,
        }
    }
}

/// `"-6.48 dB"`, `"+1.2 dB"`, `"0.988"` → the number. Tag values vary in case and spacing.
pub fn parse_number(text: &str) -> Option<f32> {
    let trimmed = text.trim();
    let number = trimmed
        .strip_suffix("dB")
        .or_else(|| trimmed.strip_suffix("db"))
        .or_else(|| trimmed.strip_suffix("DB"))
        .unwrap_or(trimmed)
        .trim();
    number.parse::<f32>().ok().filter(|v| v.is_finite())
}

/// Volume 0.0–1.0 to amplitude. Cubic, so the slider feels even to the ear rather than
/// doing all its work in the top fifth.
pub fn volume_to_amplitude(volume: f32) -> f32 {
    volume.clamp(0.0, 1.0).powi(3)
}

/// Multiplies in place and clamps to the valid sample range.
pub fn apply(samples: &mut [f32], factor: f32) {
    if factor == 1.0 {
        return;
    }
    for s in samples {
        *s = (*s * factor).clamp(-1.0, 1.0);
    }
}
