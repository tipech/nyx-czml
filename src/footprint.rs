use std::sync::Arc;

use nyx_space::cosmic::Spacecraft;

use crate::config::SensorConfig;
use crate::czml::{
    epoch_to_iso, project_to_surface_m, CzmlEllipse, CzmlMaterial, CzmlPacket, CzmlPosition,
    CzmlSampledDouble, HeightReference, InterpolationAlgorithm, ReferenceFrame, R_EARTH_KM,
};

/// Circular footprint radius (meters) for a nadir-pointing sensor on a spherical Earth.
///
/// Uses the exact spherical formula:
///   rho   = arcsin((R_E / (R_E + h)) * sin(theta))   — elevation at footprint edge
///   alpha = PI - theta - rho                           — Earth central angle
///   r     = R_E * alpha
pub fn footprint_radius_m(altitude_m: f64, half_angle_rad: f64) -> f64 {
    let r_e = R_EARTH_KM * 1000.0;
    let rho = ((r_e / (r_e + altitude_m)) * half_angle_rad.sin()).asin();
    let alpha = std::f64::consts::PI - half_angle_rad - rho;
    r_e * alpha
}

pub fn build_footprint_packet(
    object_id: &str,
    sc_name: &str,
    ecef_states: &[Spacecraft],
    sensor: &SensorConfig,
) -> CzmlPacket {
    let half_angle_rad = sensor.half_angle_deg.to_radians();
    let reference_epoch = ecef_states[0].orbit.epoch;
    let epoch_str = epoch_to_iso(reference_epoch);
    let end_str = epoch_to_iso(ecef_states.last().unwrap().orbit.epoch);
    let availability = format!("{epoch_str}/{end_str}");

    let mut surface_cartesian: Vec<f64> = Vec::with_capacity(ecef_states.len() * 4);
    let mut radius_samples: Vec<f64> = Vec::with_capacity(ecef_states.len() * 2);

    for state in ecef_states {
        let t = (state.orbit.epoch - reference_epoch).to_seconds();
        let pos = state.orbit.radius_km;

        let r_km = (pos.x * pos.x + pos.y * pos.y + pos.z * pos.z).sqrt();
        let altitude_m = (r_km - R_EARTH_KM) * 1000.0;

        let (x_m, y_m, z_m) = project_to_surface_m(pos.x, pos.y, pos.z);
        surface_cartesian.extend_from_slice(&[t, x_m, y_m, z_m]);

        radius_samples.push(t);
        radius_samples.push(footprint_radius_m(altitude_m, half_angle_rad));
    }

    let position = CzmlPosition {
        epoch: epoch_str.clone(),
        reference_frame: ReferenceFrame::Fixed,
        interpolation_algorithm: Some(InterpolationAlgorithm::Lagrange),
        interpolation_degree: Some(5),
        cartesian: Some(surface_cartesian),
        cartesian_velocity: None,
    };

    // Both axes are identical (circular footprint): share the allocation via Arc.
    let radius_arc = Arc::new(radius_samples);
    let sampled_radius = |n: Arc<Vec<f64>>| CzmlSampledDouble {
        epoch: epoch_str.clone(),
        interpolation_algorithm: Some(InterpolationAlgorithm::Linear),
        number: n,
    };

    let ellipse = CzmlEllipse {
        semi_major_axis: sampled_radius(radius_arc.clone()),
        semi_minor_axis: sampled_radius(radius_arc),
        material: CzmlMaterial::solid(sensor.color),
        height_reference: HeightReference::ClampToGround,
        show: true,
    };

    CzmlPacket {
        id: format!("{object_id}-footprint"),
        name: Some(format!("{sc_name} Footprint")),
        availability: Some(availability),
        position: Some(position),
        ellipse: Some(ellipse),
        ..Default::default()
    }
}
