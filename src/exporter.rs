use std::path::Path;
use std::sync::Arc;

use anise::constants::frames::EARTH_ITRF93;
use anise::prelude::Almanac;
use hifitime::{Duration, Epoch};
use nyx_space::cosmic::Spacecraft;
use nyx_space::md::trajectory::Traj;

use crate::config::CzmlExportCfg;
use crate::czml::{
    epoch_to_iso, CzmlColorHolder, CzmlDocument, CzmlLabel, CzmlMaterial, CzmlPacket, CzmlPath,
    CzmlPoint, CzmlPosition, InterpolationAlgorithm, ReferenceFrame,
};
use crate::error::CzmlError;
use crate::footprint::build_footprint_packet;
use crate::groundtrack::build_groundtrack_packet;

pub trait ToCzml {
    fn to_czml(&self, cfg: &CzmlExportCfg, almanac: Arc<Almanac>) -> Result<CzmlDocument, CzmlError>;
    fn to_czml_file(
        &self,
        path: &Path,
        cfg: &CzmlExportCfg,
        almanac: Arc<Almanac>,
    ) -> Result<(), CzmlError>;
}

impl ToCzml for Traj<Spacecraft> {
    fn to_czml(&self, cfg: &CzmlExportCfg, almanac: Arc<Almanac>) -> Result<CzmlDocument, CzmlError> {
        let mut doc = CzmlDocument::new(&cfg.name);

        let start = cfg.start_epoch.unwrap_or_else(|| self.first().orbit.epoch);
        let end = cfg.end_epoch.unwrap_or_else(|| self.last().orbit.epoch);

        let states = collect_states(self, cfg.step, start, end);
        if states.is_empty() {
            return Err(CzmlError::EmptyTrajectory);
        }

        let reference_epoch = states[0].orbit.epoch;
        let epoch_str = epoch_to_iso(reference_epoch);
        let end_str = epoch_to_iso(states.last().unwrap().orbit.epoch);
        let availability = format!("{epoch_str}/{end_str}");

        doc.set_clock(&epoch_str, &end_str, cfg.clock_multiplier);

        let mut cartesian_velocity: Vec<f64> = Vec::with_capacity(states.len() * 7);
        for state in &states {
            let t = (state.orbit.epoch - reference_epoch).to_seconds();
            let pos = state.orbit.radius_km;
            let vel = state.orbit.velocity_km_s;
            cartesian_velocity.extend_from_slice(&[
                t,
                pos.x * 1000.0, pos.y * 1000.0, pos.z * 1000.0,
                vel.x * 1000.0, vel.y * 1000.0, vel.z * 1000.0,
            ]);
        }

        let position = CzmlPosition {
            epoch: epoch_str.clone(),
            reference_frame: ReferenceFrame::Inertial,
            interpolation_algorithm: Some(InterpolationAlgorithm::Hermite),
            interpolation_degree: Some(5),
            cartesian_velocity: Some(cartesian_velocity),
            cartesian: None,
        };

        let path = cfg.show_path.then(|| CzmlPath {
            show: true,
            width: 1.0,
            material: CzmlMaterial::solid(cfg.path_color),
            lead_time: 0.0,
            trail_time: cfg.trail_time_s,
            resolution: 60.0,
        });

        let label = cfg.show_label.then(|| CzmlLabel {
            text: cfg.name.clone(),
            font: "11pt Lucida Console".to_string(),
            style: "FILL_AND_OUTLINE".to_string(),
            fill_color: CzmlColorHolder { rgba: [255, 255, 255, 255] },
            outline_color: CzmlColorHolder { rgba: [0, 0, 0, 255] },
            outline_width: 2.0,
            horizontal_origin: "LEFT".to_string(),
            show: true,
        });

        let point = Some(CzmlPoint {
            pixel_size: 8.0,
            color: CzmlColorHolder { rgba: cfg.path_color },
            show: true,
        });

        doc.push(CzmlPacket {
            id: cfg.object_id.clone(),
            name: Some(cfg.name.clone()),
            availability: Some(availability.clone()),
            position: Some(position),
            path,
            label,
            point,
            ..Default::default()
        });

        if cfg.show_ground_track || cfg.sensor.is_some() {
            let ecef_traj = self.to_frame(EARTH_ITRF93, almanac.clone())?;
            let ecef_states = collect_states(&ecef_traj, cfg.step, start, end);

            if cfg.show_ground_track {
                doc.push(build_groundtrack_packet(
                    &cfg.object_id,
                    &cfg.name,
                    &ecef_states,
                    cfg.trail_time_s,
                    cfg.ground_track_color,
                ));
            }
            if let Some(ref sensor) = cfg.sensor {
                doc.push(build_footprint_packet(
                    &cfg.object_id,
                    &cfg.name,
                    &ecef_states,
                    sensor,
                ));
            }
        }

        Ok(doc)
    }

    fn to_czml_file(
        &self,
        path: &Path,
        cfg: &CzmlExportCfg,
        almanac: Arc<Almanac>,
    ) -> Result<(), CzmlError> {
        let doc = self.to_czml(cfg, almanac)?;
        std::fs::write(path, doc.to_json()?)?;
        Ok(())
    }
}

fn collect_states(
    traj: &Traj<Spacecraft>,
    step: Option<Duration>,
    start: Epoch,
    end: Epoch,
) -> Vec<Spacecraft> {
    match step {
        None => traj
            .states
            .iter()
            .filter(|s| s.orbit.epoch >= start && s.orbit.epoch <= end)
            .cloned()
            .collect(),
        Some(step) => traj.every_between(step, start, end).collect(),
    }
}
