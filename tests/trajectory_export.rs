//! Integration tests: propagate a real nyx trajectory and validate CZML output.
//!
//! Ground track and footprint tests are marked `#[ignore]` because they require
//! ANISE planetary rotation data (BPC files). Run them with:
//!   cargo test -- --include-ignored
//! after the data files are available (e.g. via `MetaAlmanac::latest()`).

use std::sync::Arc;

use anise::constants::frames::EARTH_J2000;
use anise::prelude::{Almanac, Frame, Orbit};
use hifitime::{Epoch, Unit};
use nyx_space::cosmic::Spacecraft;
use nyx_space::dynamics::{OrbitalDynamics, SpacecraftDynamics};
use nyx_space::md::prelude::Propagator;
use nyx_space::md::Trajectory;
use serde_json::Value;

use nyx_czml::{CzmlExportCfg, SensorConfig, ToCzml};

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Earth's GM (km³/s²) embedded in the frame so tests need no data files.
const EARTH_GM: f64 = 398_600.435_436;

fn propagate_leo() -> (Orbit, Trajectory) {
    propagate_leo_with(Arc::new(Almanac::default()), EARTH_J2000.with_mu_km3_s2(EARTH_GM))
}

fn propagate_leo_with(almanac: Arc<Almanac>, frame: Frame) -> (Orbit, Trajectory) {
    let epoch = Epoch::from_gregorian_utc_at_midnight(2024, 1, 1);
    let orbit = Orbit::try_keplerian(6778.0, 0.0001, 45.0, 0.0, 0.0, 0.0, epoch, frame)
        .expect("valid keplerian orbit");
    let sc = Spacecraft::builder().orbit(orbit).build();
    let (_, traj) = Propagator::default(SpacecraftDynamics::new(OrbitalDynamics::two_body()))
        .with(sc, almanac)
        .for_duration_with_traj(92.0 * Unit::Minute)
        .expect("propagation succeeded");
    (orbit, traj)
}

/// Run a full export and return the parsed JSON packet array.
fn exported_packets(traj: &Trajectory, cfg: CzmlExportCfg) -> Vec<Value> {
    let almanac = Arc::new(Almanac::default());
    let doc = traj.to_czml(&cfg, almanac).expect("export succeeded");
    serde_json::from_str(&doc.to_json().expect("JSON serialized")).expect("valid JSON array")
}

// ---------------------------------------------------------------------------
// JSON structure
// ---------------------------------------------------------------------------

#[test]
fn czml_is_valid_json_array() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("Test SC"));
    assert!(!arr.is_empty());
}

#[test]
fn document_header_is_first_packet() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("My Mission"));
    assert_eq!(arr[0]["id"], "document");
    assert_eq!(arr[0]["name"], "My Mission");
    assert_eq!(arr[0]["version"], "1.0");
}

#[test]
fn spacecraft_packet_is_second() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC").without_path().without_label());
    assert_eq!(arr.len(), 2, "header + spacecraft");
    assert_eq!(arr[1]["id"], "spacecraft");
}

// ---------------------------------------------------------------------------
// Clock
// ---------------------------------------------------------------------------

#[test]
fn document_has_clock_with_correct_interval() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC"));
    let clock = &arr[0]["clock"];
    let availability = arr[1]["availability"].as_str().unwrap();
    assert_eq!(clock["interval"], availability);
    assert_eq!(clock["range"], "LOOP_STOP");
    assert_eq!(clock["step"], "SYSTEM_CLOCK_MULTIPLIER");
    assert_eq!(clock["multiplier"], 60.0);
}

#[test]
fn clock_multiplier_is_configurable() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC").with_clock_multiplier(300.0));
    assert_eq!(arr[0]["clock"]["multiplier"], 300.0);
}

// ---------------------------------------------------------------------------
// Availability interval
// ---------------------------------------------------------------------------

#[test]
fn availability_matches_trajectory_endpoints() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC"));
    let availability = arr[1]["availability"].as_str().unwrap();
    let parts: Vec<&str> = availability.split('/').collect();
    assert_eq!(parts.len(), 2);
    assert!(!parts[0].is_empty());
    assert!(!parts[1].is_empty());
    assert_ne!(parts[0], parts[1]);
}

// ---------------------------------------------------------------------------
// Position array
// ---------------------------------------------------------------------------

#[test]
fn position_reference_frame_is_inertial() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC"));
    assert_eq!(arr[1]["position"]["referenceFrame"], "INERTIAL");
}

#[test]
fn position_uses_hermite_interpolation() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC"));
    let pos = &arr[1]["position"];
    assert_eq!(pos["interpolationAlgorithm"], "HERMITE");
    assert_eq!(pos["interpolationDegree"], 5);
    assert!(pos.get("cartesianVelocity").is_some());
    assert!(pos.get("cartesian").is_none());
}

#[test]
fn cartesian_velocity_length_is_7_times_state_count() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC"));
    let cv = arr[1]["position"]["cartesianVelocity"].as_array().unwrap();
    assert_eq!(cv.len(), 7 * traj.states.len());
}

#[test]
fn first_position_offset_is_zero() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC"));
    let cv = arr[1]["position"]["cartesianVelocity"].as_array().unwrap();
    assert_eq!(cv[0].as_f64().unwrap(), 0.0);
}

#[test]
fn first_position_matches_initial_orbit_state() {
    let (initial_orbit, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC"));
    let cv = arr[1]["position"]["cartesianVelocity"].as_array().unwrap();

    let tol = 1.0; // 1 m (propagator may shift the epoch slightly)
    assert!((cv[1].as_f64().unwrap() - initial_orbit.radius_km.x * 1000.0).abs() < tol);
    assert!((cv[2].as_f64().unwrap() - initial_orbit.radius_km.y * 1000.0).abs() < tol);
    assert!((cv[3].as_f64().unwrap() - initial_orbit.radius_km.z * 1000.0).abs() < tol);
    assert!((cv[4].as_f64().unwrap() - initial_orbit.velocity_km_s.x * 1000.0).abs() < 0.01);
    assert!((cv[5].as_f64().unwrap() - initial_orbit.velocity_km_s.y * 1000.0).abs() < 0.01);
    assert!((cv[6].as_f64().unwrap() - initial_orbit.velocity_km_s.z * 1000.0).abs() < 0.01);
}

// ---------------------------------------------------------------------------
// Fixed-step sampling
// ---------------------------------------------------------------------------

#[test]
fn fixed_step_produces_uniform_sample_count() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC").with_step(60.0 * Unit::Second));
    let cv = arr[1]["position"]["cartesianVelocity"].as_array().unwrap();
    assert_eq!(cv.len() % 7, 0);
    let n = cv.len() / 7;
    assert!(n > 85 && n < 100, "expected ~93 samples at 60s step, got {n}");
}

// ---------------------------------------------------------------------------
// Optional features
// ---------------------------------------------------------------------------

#[test]
fn path_packet_present_by_default() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::default());
    assert!(arr[1].get("path").is_some());
    assert_eq!(arr[1]["path"]["show"], true);
    assert_eq!(arr[1]["path"]["leadTime"], 0.0);
}

#[test]
fn label_present_by_default() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("My SC"));
    assert!(arr[1].get("label").is_some());
    assert_eq!(arr[1]["label"]["text"], "My SC");
    assert_eq!(arr[1]["label"]["show"], true);
}

#[test]
fn without_path_omits_path_key() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC").without_path());
    assert!(arr[1].get("path").is_none());
}

#[test]
fn without_label_omits_label_key() {
    let (_, traj) = propagate_leo();
    let arr = exported_packets(&traj, CzmlExportCfg::new("SC").without_label());
    assert!(arr[1].get("label").is_none());
}

// ---------------------------------------------------------------------------
// Ground track (requires ECEF rotation data — ignored by default)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "requires ANISE BPC rotation data; run with --include-ignored after MetaAlmanac::latest()"]
fn ground_track_packet_present_when_enabled() {
    use anise::almanac::metaload::MetaAlmanac;
    let almanac = Arc::new(MetaAlmanac::latest().expect("data download"));
    let (_, traj) = propagate_leo_with(almanac.clone(), EARTH_J2000);

    let arr = exported_packets_with_almanac(
        &traj,
        CzmlExportCfg::new("SC").with_ground_track(),
        almanac,
    );
    assert_eq!(arr.len(), 3, "header + spacecraft + ground track");
    assert!(arr[2]["id"].as_str().unwrap().ends_with("-groundtrack"));
    assert_eq!(arr[2]["position"]["referenceFrame"], "FIXED");
}

#[test]
#[ignore = "requires ANISE BPC rotation data; run with --include-ignored after MetaAlmanac::latest()"]
fn footprint_packet_has_ellipse_with_positive_radii() {
    use anise::almanac::metaload::MetaAlmanac;
    let almanac = Arc::new(MetaAlmanac::latest().expect("data download"));
    let (_, traj) = propagate_leo_with(almanac.clone(), EARTH_J2000);

    let cfg = CzmlExportCfg::new("SC").with_sensor(SensorConfig {
        half_angle_deg: 60.0,
        color: [0, 100, 255, 80],
    });
    let arr = exported_packets_with_almanac(&traj, cfg, almanac);
    assert_eq!(arr.len(), 3, "header + spacecraft + footprint");
    assert!(arr[2]["id"].as_str().unwrap().ends_with("-footprint"));
    assert_eq!(arr[2]["ellipse"]["heightReference"], "CLAMP_TO_GROUND");

    let radii = arr[2]["ellipse"]["semiMajorAxis"]["number"].as_array().unwrap();
    for (i, v) in radii.iter().enumerate() {
        if i % 2 == 1 {
            assert!(v.as_f64().unwrap() > 0.0, "footprint radius must be positive");
        }
    }
}

fn exported_packets_with_almanac(
    traj: &Trajectory,
    cfg: CzmlExportCfg,
    almanac: Arc<Almanac>,
) -> Vec<Value> {
    let doc = traj.to_czml(&cfg, almanac).expect("export succeeded");
    serde_json::from_str(&doc.to_json().unwrap()).unwrap()
}
