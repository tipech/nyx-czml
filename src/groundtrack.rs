use nyx_space::cosmic::Spacecraft;

use crate::czml::{
    epoch_to_iso, project_to_surface_m, CzmlMaterial, CzmlPacket, CzmlPath, CzmlPosition,
    InterpolationAlgorithm, ReferenceFrame,
};

pub fn build_groundtrack_packet(
    object_id: &str,
    sc_name: &str,
    ecef_states: &[Spacecraft],
    trail_time_s: f64,
    color: [u8; 4],
) -> CzmlPacket {
    let reference_epoch = ecef_states[0].orbit.epoch;
    let epoch_str = epoch_to_iso(reference_epoch);
    let end_str = epoch_to_iso(ecef_states.last().unwrap().orbit.epoch);
    let availability = format!("{epoch_str}/{end_str}");

    let mut cartesian: Vec<f64> = Vec::with_capacity(ecef_states.len() * 4);
    for state in ecef_states {
        let t = (state.orbit.epoch - reference_epoch).to_seconds();
        let pos = state.orbit.radius_km;
        let (x_m, y_m, z_m) = project_to_surface_m(pos.x, pos.y, pos.z);
        cartesian.extend_from_slice(&[t, x_m, y_m, z_m]);
    }

    let position = CzmlPosition {
        epoch: epoch_str,
        reference_frame: ReferenceFrame::Fixed,
        interpolation_algorithm: Some(InterpolationAlgorithm::Lagrange),
        interpolation_degree: Some(5),
        cartesian: Some(cartesian),
        cartesian_velocity: None,
    };

    let path = CzmlPath {
        show: true,
        width: 1.5,
        material: CzmlMaterial::solid(color),
        lead_time: 0.0,
        trail_time: trail_time_s,
        resolution: 30.0,
    };

    CzmlPacket {
        id: format!("{object_id}-groundtrack"),
        name: Some(format!("{sc_name} Ground Track")),
        availability: Some(availability),
        position: Some(position),
        path: Some(path),
        ..Default::default()
    }
}
