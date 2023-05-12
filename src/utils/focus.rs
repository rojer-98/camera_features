use serde::{Deserialize, Serialize};

use diesel_db::MultipleSettingsData;
use domain::CameraId;

pub type FocusValue = f32;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FocusSettings {
    pub camera_id: CameraId,
    pub focus: TypedFocus,
    #[serde(default)]
    pub nighttime_focus: Option<TypedFocus>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
#[serde(rename_all = "lowercase")]
pub enum TypedFocus {
    Absolute(FocusValue),
    Relative(FocusValue),
    Continuous(FocusContinuous),
}

impl FocusSettings {
    pub fn new(camera_id: CameraId) -> Self {
        Self {
            camera_id,
            focus: TypedFocus::Absolute(0.),
            nighttime_focus: Default::default(),
        }
    }
}

impl MultipleSettingsData for FocusSettings {
    type Id = CameraId;

    fn get_settings_id(&self) -> Self::Id {
        self.camera_id
    }
}

#[derive(Serialize, Deserialize)]
pub struct FocusCapabilities {
    pub absolute: Option<FocusCapabilitiesAbsolute>,
    pub relative: Option<FocusCapabilitiesRelative>,
    pub continuous: Option<FocusCapabilitiesContinuous>,
}

impl Default for FocusCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusCapabilities {
    pub fn new() -> Self {
        Self {
            absolute: None,
            relative: None,
            continuous: None,
        }
    }

    pub fn absolute(mut self, min: FocusValue, max: FocusValue, step: FocusValue) -> Self {
        self.absolute = Some(FocusCapabilitiesAbsolute::new(min, max, step));
        self
    }

    pub fn continuous(mut self, min_interval: usize, max_interval: usize) -> Self {
        self.continuous = Some(FocusCapabilitiesContinuous::new(min_interval, max_interval));
        self
    }

    pub fn relative(mut self, min_step: FocusValue, max_step: FocusValue) -> Self {
        self.relative = Some(FocusCapabilitiesRelative::new(min_step, max_step));
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FocusCapabilitiesAbsolute {
    pub min: FocusValue,
    pub max: FocusValue,
    pub step: FocusValue,
}

impl FocusCapabilitiesAbsolute {
    pub fn new(min: FocusValue, max: FocusValue, step: FocusValue) -> Self {
        Self { min, max, step }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FocusCapabilitiesRelative {
    pub min_step: FocusValue,
    pub max_step: FocusValue,
}

impl FocusCapabilitiesRelative {
    pub fn new(min_step: FocusValue, max_step: FocusValue) -> Self {
        Self { min_step, max_step }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FocusCapabilitiesContinuous {
    pub min_interval: usize,
    pub max_interval: usize,
}

impl FocusCapabilitiesContinuous {
    pub fn new(min_interval: usize, max_interval: usize) -> Self {
        Self {
            min_interval,
            max_interval,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct FocusContinuous {
    pub direction: Direction,
    pub interval: usize,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Forward,
    Backward,
}
