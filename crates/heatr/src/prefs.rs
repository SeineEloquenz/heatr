//! User preferences for a bite-healer demo session.

use std::fmt;
use std::str::FromStr;

use crate::error::{HeatrError, Result};

/// Whether the person's skin is particularly sensitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkinSensitivity {
    #[default]
    Sensitive,
    Regular,
}

impl SkinSensitivity {
    /// Returns the wire encoding for this value (0-based).
    pub(crate) fn code(self) -> u8 {
        match self {
            SkinSensitivity::Sensitive => 0,
            SkinSensitivity::Regular => 1,
        }
    }
}

impl fmt::Display for SkinSensitivity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkinSensitivity::Sensitive => write!(f, "sensitive skin"),
            SkinSensitivity::Regular => write!(f, "regular skin"),
        }
    }
}

impl FromStr for SkinSensitivity {
    type Err = HeatrError;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "sensitive" => Ok(SkinSensitivity::Sensitive),
            "regular" => Ok(SkinSensitivity::Regular),
            _ => Err(HeatrError::InvalidPreference {
                field: "skin_sensitivity".into(),
                value: s.into(),
                valid: "sensitive, regular".into(),
            }),
        }
    }
}

/// The age cohort of a person.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Generation {
    #[default]
    Child,
    Adult,
}

impl Generation {
    /// Returns the wire encoding for this value (0-based), shifted left by 1.
    pub(crate) fn code(self) -> u8 {
        match self {
            Generation::Child => 0,
            Generation::Adult => 1,
        }
    }
}

impl fmt::Display for Generation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Generation::Child => write!(f, "child"),
            Generation::Adult => write!(f, "adult"),
        }
    }
}

impl FromStr for Generation {
    type Err = HeatrError;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "child" => Ok(Generation::Child),
            "adult" => Ok(Generation::Adult),
            _ => Err(HeatrError::InvalidPreference {
                field: "generation".into(),
                value: s.into(),
                valid: "child, adult".into(),
            }),
        }
    }
}

/// The duration of a demo session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Duration {
    #[default]
    Short,
    Medium,
    Long,
}

impl Duration {
    /// Returns the 0-based wire encoding for this value.
    pub(crate) fn code(self) -> u8 {
        match self {
            Duration::Short => 0,
            Duration::Medium => 1,
            Duration::Long => 2,
        }
    }
}

impl fmt::Display for Duration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Duration::Short => write!(f, "short duration"),
            Duration::Medium => write!(f, "medium duration"),
            Duration::Long => write!(f, "long duration"),
        }
    }
}

impl FromStr for Duration {
    type Err = HeatrError;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "short" => Ok(Duration::Short),
            "medium" => Ok(Duration::Medium),
            "long" => Ok(Duration::Long),
            _ => Err(HeatrError::InvalidPreference {
                field: "duration".into(),
                value: s.into(),
                valid: "short, medium, long".into(),
            }),
        }
    }
}

/// User preferences for a bite healer demo session.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Preferences {
    pub duration: Duration,
    pub generation: Generation,
    pub skin_sensitivity: SkinSensitivity,
}

impl fmt::Display for Preferences {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}, {}, {}",
            self.duration, self.generation, self.skin_sensitivity
        )
    }
}
