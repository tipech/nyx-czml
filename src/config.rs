use hifitime::{Duration, Epoch};

/// Configuration for CZML export.
#[derive(Debug, Clone)]
pub struct CzmlExportCfg {
    /// Display name for the spacecraft and document.
    pub name: String,
    /// CZML packet ID. Must be unique within the document.
    pub object_id: String,
    /// Sampling interval.
    /// - `None`: emit raw propagator knots (most compact, HERMITE interpolation).
    /// - `Some(step)`: resample at uniform intervals (predictable density, still HERMITE).
    pub step: Option<Duration>,
    /// Optional start epoch override (defaults to trajectory start).
    pub start_epoch: Option<Epoch>,
    /// Optional end epoch override (defaults to trajectory end).
    pub end_epoch: Option<Epoch>,
    /// Render the orbital trail behind the spacecraft.
    pub show_path: bool,
    /// Duration of the orbital trail shown behind the spacecraft, in seconds.
    pub trail_time_s: f64,
    /// RGBA color of the orbital trail and spacecraft point.
    pub path_color: [u8; 4],
    /// Render the spacecraft name as a text label in Cesium.
    pub show_label: bool,
    /// Generate a ground track (sub-satellite path on Earth surface) packet.
    /// Requires an `Almanac` passed to `to_czml`.
    pub show_ground_track: bool,
    /// RGBA color of the ground track.
    pub ground_track_color: [u8; 4],
    /// If set, generate a sensor footprint ellipse packet.
    pub sensor: Option<SensorConfig>,
    /// Cesium timeline playback speed as a real-time multiplier (e.g. 60 = 60× faster).
    pub clock_multiplier: f64,
}

/// Circular, nadir-pointing sensor footprint configuration.
#[derive(Debug, Clone)]
pub struct SensorConfig {
    /// Half-angle of the sensor cone in degrees.
    pub half_angle_deg: f64,
    /// Fill color of the footprint ellipse (semi-transparent recommended).
    pub color: [u8; 4],
}

impl Default for CzmlExportCfg {
    fn default() -> Self {
        Self {
            name: "Spacecraft".to_string(),
            object_id: "spacecraft".to_string(),
            step: None,
            start_epoch: None,
            end_epoch: None,
            show_path: true,
            trail_time_s: 5400.0, // 90 minutes
            path_color: [255, 255, 0, 200],
            show_label: true,
            show_ground_track: false,
            ground_track_color: [0, 200, 100, 200],
            sensor: None,
            clock_multiplier: 60.0,
        }
    }
}

impl CzmlExportCfg {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn with_step(mut self, step: Duration) -> Self {
        self.step = Some(step);
        self
    }

    pub fn with_time_window(mut self, start: Epoch, end: Epoch) -> Self {
        self.start_epoch = Some(start);
        self.end_epoch = Some(end);
        self
    }

    pub fn with_path_color(mut self, rgba: [u8; 4]) -> Self {
        self.path_color = rgba;
        self
    }

    pub fn with_ground_track(mut self) -> Self {
        self.show_ground_track = true;
        self
    }

    pub fn with_ground_track_color(mut self, rgba: [u8; 4]) -> Self {
        self.ground_track_color = rgba;
        self
    }

    pub fn with_sensor(mut self, sensor: SensorConfig) -> Self {
        self.sensor = Some(sensor);
        self
    }

    pub fn without_path(mut self) -> Self {
        self.show_path = false;
        self
    }

    pub fn without_label(mut self) -> Self {
        self.show_label = false;
        self
    }

    pub fn with_trail_time(mut self, seconds: f64) -> Self {
        self.trail_time_s = seconds;
        self
    }

    pub fn with_clock_multiplier(mut self, multiplier: f64) -> Self {
        self.clock_multiplier = multiplier;
        self
    }
}
