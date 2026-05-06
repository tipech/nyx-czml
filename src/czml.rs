use std::sync::Arc;

use hifitime::{Epoch, TimeScale};
use serde::Serialize;

pub(crate) const R_EARTH_M: f64 = 6_371_000.0;
pub(crate) const R_EARTH_KM: f64 = R_EARTH_M / 1000.0;

pub(crate) fn epoch_to_iso(epoch: Epoch) -> String {
    epoch.to_time_scale(TimeScale::UTC).to_isoformat()
}

/// Project an ECEF position (km) radially onto the Earth surface, returning meters.
pub(crate) fn project_to_surface_m(x_km: f64, y_km: f64, z_km: f64) -> (f64, f64, f64) {
    let r_km = (x_km * x_km + y_km * y_km + z_km * z_km).sqrt();
    let scale = R_EARTH_KM / r_km * 1000.0;
    (x_km * scale, y_km * scale, z_km * scale)
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub enum ReferenceFrame {
    #[serde(rename = "INERTIAL")]
    Inertial,
    #[serde(rename = "FIXED")]
    Fixed,
}

#[derive(Debug, Clone, Serialize)]
pub enum InterpolationAlgorithm {
    #[serde(rename = "HERMITE")]
    Hermite,
    #[serde(rename = "LAGRANGE")]
    Lagrange,
    #[serde(rename = "LINEAR")]
    Linear,
}

#[derive(Debug, Clone, Serialize)]
pub enum HeightReference {
    #[serde(rename = "CLAMP_TO_GROUND")]
    ClampToGround,
    #[serde(rename = "RELATIVE_TO_GROUND")]
    RelativeToGround,
}

// ---------------------------------------------------------------------------
// Clock
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub enum ClockRange {
    #[serde(rename = "LOOP_STOP")]
    LoopStop,
    #[serde(rename = "LOOP_RESTART")]
    LoopRestart,
    #[serde(rename = "CLAMPED")]
    Clamped,
}

#[derive(Debug, Clone, Serialize)]
pub enum ClockStep {
    #[serde(rename = "SYSTEM_CLOCK_MULTIPLIER")]
    SystemClockMultiplier,
    #[serde(rename = "TICK_DEPENDENT")]
    TickDependent,
    #[serde(rename = "SYSTEM_CLOCK")]
    SystemClock,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlClock {
    pub interval: String,
    pub current_time: String,
    pub multiplier: f64,
    pub range: ClockRange,
    pub step: ClockStep,
}

// ---------------------------------------------------------------------------
// Document
// ---------------------------------------------------------------------------

/// A CZML document is a JSON array of packets. The first packet is always the document header.
#[derive(Debug, Clone, Serialize)]
pub struct CzmlDocument(pub Vec<CzmlPacket>);

impl CzmlDocument {
    pub fn new(name: impl Into<String>) -> Self {
        let header = CzmlPacket {
            id: "document".to_string(),
            name: Some(name.into()),
            version: Some("1.0".to_string()),
            ..Default::default()
        };
        CzmlDocument(vec![header])
    }

    /// Set the timeline clock on the document header packet.
    ///
    /// `start` and `end` are ISO 8601 strings matching the trajectory's availability interval.
    /// `multiplier` controls Cesium's playback speed relative to wall time (e.g. 60 = 60× faster).
    pub fn set_clock(&mut self, start: &str, end: &str, multiplier: f64) {
        self.0[0].clock = Some(CzmlClock {
            interval: format!("{start}/{end}"),
            current_time: start.to_string(),
            multiplier,
            range: ClockRange::LoopStop,
            step: ClockStep::SystemClockMultiplier,
        });
    }

    pub fn push(&mut self, packet: CzmlPacket) {
        self.0.push(packet);
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(&self.0)
    }

    pub fn to_json_compact(&self) -> serde_json::Result<String> {
        serde_json::to_string(&self.0)
    }
}

// ---------------------------------------------------------------------------
// Packets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CzmlPacket {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// ISO 8601 interval: "2024-01-01T00:00:00Z/2024-01-02T00:00:00Z"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub availability: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clock: Option<CzmlClock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<CzmlPosition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<CzmlPath>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<CzmlLabel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ellipse: Option<CzmlEllipse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub point: Option<CzmlPoint>,
}

// ---------------------------------------------------------------------------
// Position
// ---------------------------------------------------------------------------

/// Time-sampled Cartesian position in meters.
///
/// For HERMITE: use `cartesian_velocity` with 7 values per sample [t, x, y, z, vx, vy, vz].
/// For LAGRANGE/LINEAR: use `cartesian` with 4 values per sample [t, x, y, z].
/// Time values are seconds offset from `epoch`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlPosition {
    /// ISO 8601 reference epoch. All time offsets are seconds from this epoch.
    pub epoch: String,
    pub reference_frame: ReferenceFrame,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpolation_algorithm: Option<InterpolationAlgorithm>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpolation_degree: Option<u32>,
    /// [t0, x0, y0, z0, vx0, vy0, vz0, t1, ...] — used with HERMITE
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cartesian_velocity: Option<Vec<f64>>,
    /// [t0, x0, y0, z0, t1, x1, y1, z1, ...] — used with LAGRANGE/LINEAR
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cartesian: Option<Vec<f64>>,
}

// ---------------------------------------------------------------------------
// Path, label, point
// ---------------------------------------------------------------------------

/// Orbit trail rendered behind (and optionally ahead of) the spacecraft.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlPath {
    pub show: bool,
    pub width: f64,
    pub material: CzmlMaterial,
    /// Seconds of future trajectory to show (0 = no lead).
    pub lead_time: f64,
    /// Seconds of past trajectory to show.
    pub trail_time: f64,
    /// Cesium rendering resolution in seconds.
    pub resolution: f64,
}

/// Text label attached to the spacecraft in Cesium.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlLabel {
    pub text: String,
    pub font: String,
    pub style: String,
    pub fill_color: CzmlColorHolder,
    pub outline_color: CzmlColorHolder,
    pub outline_width: f64,
    pub horizontal_origin: String,
    pub show: bool,
}

/// Small dot rendered at the spacecraft position.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlPoint {
    pub pixel_size: f64,
    pub color: CzmlColorHolder,
    pub show: bool,
}

// ---------------------------------------------------------------------------
// Ellipse (sensor footprint)
// ---------------------------------------------------------------------------

/// Sensor footprint ellipse, clamped to the Earth surface.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlEllipse {
    pub semi_major_axis: CzmlSampledDouble,
    pub semi_minor_axis: CzmlSampledDouble,
    pub material: CzmlMaterial,
    pub height_reference: HeightReference,
    pub show: bool,
}

/// A time-sampled scalar value: [t0, v0, t1, v1, ...].
///
/// The inner `number` array is `Arc`-wrapped so that two fields (e.g.
/// `semi_major_axis` and `semi_minor_axis` on a circular footprint) can share
/// the same allocation without copying.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlSampledDouble {
    pub epoch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpolation_algorithm: Option<InterpolationAlgorithm>,
    /// Alternating [time_offset_s, value] pairs.
    pub number: Arc<Vec<f64>>,
}

// ---------------------------------------------------------------------------
// Material / color
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CzmlMaterial {
    pub solid_color: CzmlSolidColor,
}

impl CzmlMaterial {
    pub fn solid(rgba: [u8; 4]) -> Self {
        CzmlMaterial {
            solid_color: CzmlSolidColor {
                color: CzmlColorHolder { rgba },
            },
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CzmlSolidColor {
    pub color: CzmlColorHolder,
}

/// CZML color as an RGBA array [r, g, b, a] with values 0–255.
#[derive(Debug, Clone, Serialize)]
pub struct CzmlColorHolder {
    pub rgba: [u8; 4],
}
