use nyx_czml::czml::{CzmlDocument, CzmlMaterial, CzmlPacket, CzmlPath, CzmlPosition};
use nyx_czml::{InterpolationAlgorithm, ReferenceFrame};
use serde_json::Value;

// --- CzmlDocument ---

#[test]
fn document_starts_with_header() {
    let doc = CzmlDocument::new("My Mission");
    let json = doc.to_json().unwrap();
    let arr: Vec<Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(arr[0]["id"], "document");
    assert_eq!(arr[0]["name"], "My Mission");
    assert_eq!(arr[0]["version"], "1.0");
}

#[test]
fn push_adds_packet_after_header() {
    let mut doc = CzmlDocument::new("Test");
    doc.push(CzmlPacket {
        id: "sat-1".to_string(),
        name: Some("Satellite".to_string()),
        ..Default::default()
    });
    let json = doc.to_json().unwrap();
    let arr: Vec<Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[1]["id"], "sat-1");
}

#[test]
fn compact_json_is_valid() {
    let doc = CzmlDocument::new("Compact");
    let compact = doc.to_json_compact().unwrap();
    // No newlines in compact mode
    assert!(!compact.contains('\n'));
    // Still valid JSON
    let _: Vec<Value> = serde_json::from_str(&compact).unwrap();
}

// --- Null fields omitted ---

#[test]
fn none_fields_omitted_from_packet() {
    let packet = CzmlPacket {
        id: "sc".to_string(),
        name: Some("SC".to_string()),
        ..Default::default()
    };
    let val: Value = serde_json::to_value(&packet).unwrap();
    // None fields must be absent from JSON — Cesium is strict about unknown keys
    assert!(val.get("version").is_none());
    assert!(val.get("availability").is_none());
    assert!(val.get("clock").is_none());
    assert!(val.get("position").is_none());
    assert!(val.get("path").is_none());
    assert!(val.get("label").is_none());
    assert!(val.get("ellipse").is_none());
    assert!(val.get("point").is_none());
}

#[test]
fn position_omits_cartesian_when_using_cartesian_velocity() {
    let pos = CzmlPosition {
        epoch: "2024-01-01T00:00:00Z".to_string(),
        reference_frame: ReferenceFrame::Inertial,
        interpolation_algorithm: Some(InterpolationAlgorithm::Hermite),
        interpolation_degree: Some(5),
        cartesian_velocity: Some(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        cartesian: None,
    };
    let val: Value = serde_json::to_value(&pos).unwrap();
    assert!(val.get("cartesianVelocity").is_some());
    assert!(val.get("cartesian").is_none());
}

// --- camelCase field names ---

#[test]
fn position_uses_camel_case_keys() {
    let pos = CzmlPosition {
        epoch: "2024-01-01T00:00:00Z".to_string(),
        reference_frame: ReferenceFrame::Inertial,
        interpolation_algorithm: Some(InterpolationAlgorithm::Hermite),
        interpolation_degree: Some(5),
        cartesian_velocity: Some(vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]),
        cartesian: None,
    };
    let val: Value = serde_json::to_value(&pos).unwrap();
    assert!(val.get("referenceFrame").is_some(), "snake_case should be camelCase");
    assert!(val.get("interpolationAlgorithm").is_some());
    assert!(val.get("interpolationDegree").is_some());
    assert!(val.get("cartesianVelocity").is_some());
    // snake_case keys must not appear
    assert!(val.get("reference_frame").is_none());
    assert!(val.get("interpolation_algorithm").is_none());
}

#[test]
fn path_uses_camel_case_keys() {
    let path = CzmlPath {
        show: true,
        width: 1.5,
        material: CzmlMaterial::solid([255, 255, 0, 200]),
        lead_time: 0.0,
        trail_time: 5400.0,
        resolution: 60.0,
    };
    let val: Value = serde_json::to_value(&path).unwrap();
    assert!(val.get("leadTime").is_some());
    assert!(val.get("trailTime").is_some());
    assert!(val.get("lead_time").is_none());
    assert!(val.get("trail_time").is_none());
}

// --- CZML position array layout ---

#[test]
fn hermite_array_has_7_values_per_state() {
    let two_states = vec![
        0.0,  1000.0, 2000.0, 3000.0, 100.0, 200.0, 300.0,  // state 0
        60.0, 1100.0, 2100.0, 3100.0, 110.0, 210.0, 310.0,  // state 1
    ];
    let pos = CzmlPosition {
        epoch: "2024-01-01T00:00:00Z".to_string(),
        reference_frame: ReferenceFrame::Inertial,
        interpolation_algorithm: Some(InterpolationAlgorithm::Hermite),
        interpolation_degree: Some(5),
        cartesian_velocity: Some(two_states),
        cartesian: None,
    };
    let val: Value = serde_json::to_value(&pos).unwrap();
    let arr = val["cartesianVelocity"].as_array().unwrap();
    assert_eq!(arr.len(), 14, "2 states × 7 values each");
    // First value is time offset = 0.0
    assert_eq!(arr[0].as_f64().unwrap(), 0.0);
    // Second value is x position in meters
    assert_eq!(arr[1].as_f64().unwrap(), 1000.0);
}

#[test]
fn material_solid_color_rgba() {
    let mat = CzmlMaterial::solid([255, 128, 0, 200]);
    let val: Value = serde_json::to_value(&mat).unwrap();
    let rgba = &val["solidColor"]["color"]["rgba"];
    assert_eq!(rgba[0].as_u64().unwrap(), 255);
    assert_eq!(rgba[1].as_u64().unwrap(), 128);
    assert_eq!(rgba[2].as_u64().unwrap(), 0);
    assert_eq!(rgba[3].as_u64().unwrap(), 200);
}

// --- Clock ---

#[test]
fn clock_on_document_header() {
    let mut doc = CzmlDocument::new("Test");
    doc.set_clock("2024-01-01T00:00:00Z", "2024-01-02T00:00:00Z", 60.0);
    let arr: Vec<Value> = serde_json::from_str(&doc.to_json().unwrap()).unwrap();
    let clock = &arr[0]["clock"];
    assert_eq!(clock["interval"], "2024-01-01T00:00:00Z/2024-01-02T00:00:00Z");
    assert_eq!(clock["currentTime"], "2024-01-01T00:00:00Z");
    assert_eq!(clock["multiplier"], 60.0);
    assert_eq!(clock["range"], "LOOP_STOP");
    assert_eq!(clock["step"], "SYSTEM_CLOCK_MULTIPLIER");
}

#[test]
fn clock_absent_when_not_set() {
    let doc = CzmlDocument::new("Test");
    let arr: Vec<Value> = serde_json::from_str(&doc.to_json().unwrap()).unwrap();
    assert!(arr[0].get("clock").is_none());
}

// --- Config defaults ---

#[test]
fn export_cfg_defaults() {
    use nyx_czml::CzmlExportCfg;
    let cfg = CzmlExportCfg::default();
    assert!(cfg.show_path);
    assert!(cfg.show_label);
    assert!(!cfg.show_ground_track);
    assert!(cfg.sensor.is_none());
    assert!(cfg.step.is_none(), "default should be raw knots");
    assert_eq!(cfg.trail_time_s, 5400.0);
}

#[test]
fn export_cfg_builder_methods() {
    use hifitime::Unit;
    use nyx_czml::{CzmlExportCfg, SensorConfig};

    let cfg = CzmlExportCfg::new("Sat")
        .with_step(60.0 * Unit::Second)
        .with_ground_track()
        .with_trail_time(3600.0)
        .without_label()
        .with_sensor(SensorConfig {
            half_angle_deg: 45.0,
            color: [255, 0, 0, 100],
        });

    assert_eq!(cfg.name, "Sat");
    assert!(cfg.step.is_some());
    assert!(cfg.show_ground_track);
    assert_eq!(cfg.trail_time_s, 3600.0);
    assert!(!cfg.show_label);
    assert!(cfg.sensor.is_some());
    assert_eq!(cfg.sensor.unwrap().half_angle_deg, 45.0);
}
