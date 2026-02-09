use macroquad::prelude::{vec3, Vec3};

use crate::config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisualQuality {
    Low,
    Medium,
    High,
    Ultra,
}

impl VisualQuality {
    pub const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::Ultra];

    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low",
            Self::Medium => "Medium",
            Self::High => "High",
            Self::Ultra => "Ultra",
        }
    }

    pub fn parse_cli(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "low" => Some(Self::Low),
            "med" | "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            "ultra" => Some(Self::Ultra),
            _ => None,
        }
    }

    pub fn rank(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Ultra => 3,
        }
    }

    pub fn from_rank(rank: u8) -> Self {
        match rank {
            0 => Self::Low,
            1 => Self::Medium,
            2 => Self::High,
            _ => Self::Ultra,
        }
    }

    pub fn lower(self) -> Self {
        Self::from_rank(self.rank().saturating_sub(1))
    }

    pub fn higher(self) -> Self {
        Self::from_rank((self.rank() + 1).min(Self::Ultra.rank()))
    }

    pub fn clamp_between(self, min: Self, max: Self) -> Self {
        let lo = min.rank().min(max.rank());
        let hi = min.rank().max(max.rank());
        Self::from_rank(self.rank().clamp(lo, hi))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VisualSettings {
    pub quality: VisualQuality,
    pub atmosphere_enabled: bool,
    pub storm_fx_enabled: bool,
    pub creature_detail_enabled: bool,
    pub trails_enabled: bool,
    pub shelter_highlight_enabled: bool,
}

impl Default for VisualSettings {
    fn default() -> Self {
        Self::with_quality(VisualQuality::High)
    }
}

impl VisualSettings {
    pub fn with_quality(quality: VisualQuality) -> Self {
        let mut settings = Self {
            quality,
            atmosphere_enabled: true,
            storm_fx_enabled: true,
            creature_detail_enabled: true,
            trails_enabled: true,
            shelter_highlight_enabled: true,
        };

        if quality == VisualQuality::Low {
            settings.trails_enabled = false;
        }

        settings
    }

    pub fn set_quality_preset(&mut self, quality: VisualQuality) {
        *self = Self::with_quality(quality);
    }

    pub fn set_quality_only(&mut self, quality: VisualQuality) {
        self.quality = quality;
    }

    pub fn storm_line_cap(self) -> usize {
        match self.quality {
            VisualQuality::Low => config::VISUAL_STORM_LINES_LOW,
            VisualQuality::Medium => config::VISUAL_STORM_LINES_MEDIUM,
            VisualQuality::High => config::VISUAL_STORM_LINES_HIGH,
            VisualQuality::Ultra => config::VISUAL_STORM_LINES_ULTRA,
        }
    }

    pub fn storm_gust_cap(self) -> usize {
        match self.quality {
            VisualQuality::Low => config::VISUAL_STORM_GUSTS_LOW,
            VisualQuality::Medium => config::VISUAL_STORM_GUSTS_MEDIUM,
            VisualQuality::High => config::VISUAL_STORM_GUSTS_HIGH,
            VisualQuality::Ultra => config::VISUAL_STORM_GUSTS_ULTRA,
        }
    }

    pub fn shelter_band_count(self) -> usize {
        match self.quality {
            VisualQuality::Low => config::VISUAL_SHELTER_BANDS_LOW,
            VisualQuality::Medium => config::VISUAL_SHELTER_BANDS_MEDIUM,
            VisualQuality::High => config::VISUAL_SHELTER_BANDS_HIGH,
            VisualQuality::Ultra => config::VISUAL_SHELTER_BANDS_ULTRA,
        }
    }

    pub fn atmosphere_strength(self) -> f32 {
        let quality_mult = match self.quality {
            VisualQuality::Low => 0.75,
            VisualQuality::Medium => 0.9,
            VisualQuality::High => 1.0,
            VisualQuality::Ultra => 1.15,
        };
        config::VISUAL_ATMOSPHERE_BASE_STRENGTH * quality_mult
    }

    pub fn bloom_threshold(self) -> f32 {
        match self.quality {
            VisualQuality::Low => config::VISUAL_BLOOM_THRESHOLD_LOW,
            VisualQuality::Medium => config::VISUAL_BLOOM_THRESHOLD_MEDIUM,
            VisualQuality::High => config::VISUAL_BLOOM_THRESHOLD_HIGH,
            VisualQuality::Ultra => config::VISUAL_BLOOM_THRESHOLD_ULTRA,
        }
    }

    pub fn bloom_intensity(self) -> f32 {
        match self.quality {
            VisualQuality::Low => config::VISUAL_BLOOM_INTENSITY_LOW,
            VisualQuality::Medium => config::VISUAL_BLOOM_INTENSITY_MEDIUM,
            VisualQuality::High => config::VISUAL_BLOOM_INTENSITY_HIGH,
            VisualQuality::Ultra => config::VISUAL_BLOOM_INTENSITY_ULTRA,
        }
    }

    pub fn vignette_strength(self) -> f32 {
        match self.quality {
            VisualQuality::Low => config::VISUAL_VIGNETTE_LOW,
            VisualQuality::Medium => config::VISUAL_VIGNETTE_MEDIUM,
            VisualQuality::High => config::VISUAL_VIGNETTE_HIGH,
            VisualQuality::Ultra => config::VISUAL_VIGNETTE_ULTRA,
        }
    }

    pub fn grade_strength(self) -> f32 {
        match self.quality {
            VisualQuality::Low => config::VISUAL_GRADE_STRENGTH_LOW,
            VisualQuality::Medium => config::VISUAL_GRADE_STRENGTH_MEDIUM,
            VisualQuality::High => config::VISUAL_GRADE_STRENGTH_HIGH,
            VisualQuality::Ultra => config::VISUAL_GRADE_STRENGTH_ULTRA,
        }
    }

    pub fn grade_tint(self) -> Vec3 {
        match self.quality {
            VisualQuality::Low => vec3(0.98, 1.0, 1.02),
            VisualQuality::Medium => vec3(0.97, 1.0, 1.03),
            VisualQuality::High => vec3(0.96, 1.0, 1.05),
            VisualQuality::Ultra => vec3(0.95, 1.0, 1.07),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisualQualityBounds {
    pub min: VisualQuality,
    pub max: VisualQuality,
}

impl VisualQualityBounds {
    pub fn new(min: VisualQuality, max: VisualQuality) -> Self {
        if min.rank() <= max.rank() {
            Self { min, max }
        } else {
            Self { min: max, max: min }
        }
    }

    pub fn clamp(&self, quality: VisualQuality) -> VisualQuality {
        quality.clamp_between(self.min, self.max)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdaptiveQualityConfig {
    pub degrade_frames: u32,
    pub upgrade_frames: u32,
}

impl Default for AdaptiveQualityConfig {
    fn default() -> Self {
        Self {
            degrade_frames: 6,
            upgrade_frames: 110,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AdaptiveQualityController {
    bounds: VisualQualityBounds,
    cfg: AdaptiveQualityConfig,
    over_budget_frames: u32,
    under_budget_frames: u32,
}

impl AdaptiveQualityController {
    pub fn new(bounds: VisualQualityBounds, cfg: AdaptiveQualityConfig) -> Self {
        Self {
            bounds,
            cfg,
            over_budget_frames: 0,
            under_budget_frames: 0,
        }
    }

    pub fn set_bounds(&mut self, bounds: VisualQualityBounds) {
        self.bounds = bounds;
    }

    pub fn observe(
        &mut self,
        current: VisualQuality,
        frame_ms: f32,
        target_frame_ms: f32,
    ) -> VisualQuality {
        let clamped_current = self.bounds.clamp(current);
        let degrade_threshold = target_frame_ms * 1.08;
        let upgrade_threshold = target_frame_ms * 0.75;

        if frame_ms > degrade_threshold {
            self.over_budget_frames = self.over_budget_frames.saturating_add(1);
            self.under_budget_frames = 0;
        } else if frame_ms < upgrade_threshold {
            self.under_budget_frames = self.under_budget_frames.saturating_add(1);
            self.over_budget_frames = 0;
        } else {
            self.over_budget_frames = 0;
            self.under_budget_frames = 0;
        }

        if self.over_budget_frames >= self.cfg.degrade_frames {
            self.over_budget_frames = 0;
            self.under_budget_frames = 0;
            return self.bounds.clamp(clamped_current.lower());
        }

        if self.under_budget_frames >= self.cfg.upgrade_frames {
            self.under_budget_frames = 0;
            self.over_budget_frames = 0;
            return self.bounds.clamp(clamped_current.higher());
        }

        clamped_current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_clamp_between_bounds() {
        let bounds = VisualQualityBounds::new(VisualQuality::Medium, VisualQuality::High);
        assert_eq!(bounds.clamp(VisualQuality::Low), VisualQuality::Medium);
        assert_eq!(bounds.clamp(VisualQuality::Ultra), VisualQuality::High);
    }

    #[test]
    fn adaptive_quality_downgrades_fast_and_upgrades_slow() {
        let bounds = VisualQualityBounds::new(VisualQuality::Low, VisualQuality::Ultra);
        let cfg = AdaptiveQualityConfig {
            degrade_frames: 2,
            upgrade_frames: 3,
        };
        let mut ctrl = AdaptiveQualityController::new(bounds, cfg);
        let mut q = VisualQuality::High;

        q = ctrl.observe(q, 25.0, 16.7);
        assert_eq!(q, VisualQuality::High);
        q = ctrl.observe(q, 24.0, 16.7);
        assert_eq!(q, VisualQuality::Medium);

        q = ctrl.observe(q, 8.0, 16.7);
        assert_eq!(q, VisualQuality::Medium);
        q = ctrl.observe(q, 8.0, 16.7);
        assert_eq!(q, VisualQuality::Medium);
        q = ctrl.observe(q, 8.0, 16.7);
        assert_eq!(q, VisualQuality::High);
    }
}
