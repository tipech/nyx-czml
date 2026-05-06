//! Propagate a 400 km sun-synchronous orbit for 3 hours and export as CZML.
//!
//! On first run, ANISE data files are downloaded automatically (~60 MB total) and
//! cached in your OS data directory. Subsequent runs are fully offline.
//! For details: https://github.com/nyx-space/anise/blob/master/data/latest.dhall
//!
//! Load the resulting `leo.czml` in CesiumJS Sandcastle:
//! ```js
//! const dataSource = await Cesium.CzmlDataSource.load('leo.czml');
//! viewer.dataSources.add(dataSource);
//! viewer.zoomTo(dataSource);
//! ```

use std::path::Path;
use std::sync::Arc;

use anise::constants::frames::EARTH_J2000;
use anise::prelude::Orbit;
use hifitime::{Epoch, Unit};
use nyx_space::cosmic::{MetaAlmanac, Spacecraft};
use nyx_space::dynamics::{OrbitalDynamics, SpacecraftDynamics};
use nyx_space::md::prelude::Propagator;

use nyx_czml::{CzmlExportCfg, SensorConfig, ToCzml};

fn main() -> anyhow::Result<()> {
    // Downloads DE440s, PCK11, Moon/Earth orientation kernels on first run; cached after.
    let almanac = Arc::new(MetaAlmanac::latest().map_err(Box::new)?);
    let earth = almanac.frame_info(EARTH_J2000)?;

    // 400 km circular orbit, ~98° inclination (sun-synchronous)
    let epoch = Epoch::from_gregorian_utc_at_midnight(2024, 1, 1);
    let orbit = Orbit::try_keplerian(
        6778.0, // semi-major axis km (≈ 400 km altitude)
        0.001,  // eccentricity
        98.2,   // inclination deg (sun-synchronous)
        0.0,    // RAAN deg
        0.0,    // AoP deg
        0.0,    // true anomaly deg
        epoch,
        earth,
    )?;

    let sc = Spacecraft::builder().orbit(orbit).build();

    let (_, traj) = Propagator::default(SpacecraftDynamics::new(OrbitalDynamics::two_body()))
        .with(sc, almanac.clone())
        .for_duration_with_traj(3.0 * Unit::Hour)?;

    println!(
        "Propagated {} states ({} to {})",
        traj.states.len(),
        traj.first().orbit.epoch,
        traj.last().orbit.epoch,
    );

    let cfg = CzmlExportCfg::new("LEO Spacecraft")
        .with_step(60.0 * Unit::Second)
        .with_trail_time(5400.0) // 90-minute trail (one full orbit)
        .with_ground_track()
        .with_sensor(SensorConfig {
            half_angle_deg: 30.0,
            color: [0, 100, 255, 80],
        });

    let output = Path::new("leo.czml");
    traj.to_czml_file(output, &cfg, almanac)?;
    println!("Wrote {}", output.display());

    Ok(())
}
