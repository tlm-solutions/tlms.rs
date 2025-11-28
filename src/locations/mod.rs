pub mod gps;
pub mod region;
mod tests;
pub mod waypoint;

use crate::schema::*;

use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Version of the [`LocationsJson`] shcema used.
pub const SCHEMA: &str = "3"; // INCREMENT ME ON ANY BREAKING CHANGE!!!!11111one

/// maximum distance in meters
pub const SANE_INTERPOLATION_DISTANCE: i32 = 50;
/// Mean earth radius, required for calcuation of distances between the GPS points
pub const MEAN_EARTH_RADIUS: u32 = 6_371_000;

/// name of the constraint in the r09_transmission_locations, for unique combination of region and
/// transmission position.
pub const REGION_POSITION_UNIQUE_CONSTRAINT: &str = "unique_region_position";

/// This struct is used to query R09 telegram transmission positions from the database. Every entry
/// corresponds to unique transmission location, that is inferred over multiple measurements. For
/// raw per-measurement data see [`TransmissionLocationRaw`]
#[derive(Debug, Clone, Serialize, Queryable, Identifiable, AsChangeset, ToSchema)]
#[diesel(table_name = r09_transmission_locations)]
pub struct TransmissionLocation {
    /// Primary key
    pub id: i64,
    /// ID of the region where telegram was transmitted
    pub region: i64,
    /// Reporting Point inside the r09 telegram (*meldepunkt*) ID
    pub reporting_point: i32,
    /// Report location latitude
    pub lat: f64,
    /// Report location longitude
    pub lon: f64,
    /// If this transmission postion inserted from absolute data, and all the inference for it
    /// should be ignored
    pub ground_truth: bool,
}

/// This struct is used to insert R09 telegram transmission positions to the database. Every entry
/// corresponds to unique transmission location, that is inferred over multiple measurements. For
/// raw per-measurement data see [`InsertTransmissionLocationRaw`]
#[derive(Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = r09_transmission_locations)]
pub struct InsertTransmissionLocation {
    /// Primary key. During INSERT should be [`None`] so DB can auto-increment it
    pub id: Option<i64>,
    /// ID of the region where telegram was transmitted
    pub region: i64,
    /// Reporting Point inside the r09 telegram (*meldepunkt*) ID
    pub reporting_point: i32,
    /// Report location latitude
    pub lat: f64,
    /// Report location longitude
    pub lon: f64,
    /// If this transmission postion inserted from absolute data, and all the inference for it
    /// should be ignored
    pub ground_truth: bool,
}

/// This struct queries the database for transmission locations inferred from every single trekkie
/// run. This is useful if you want to refine the position of [`TransmissionLocation`]
#[derive(Debug, Clone, Serialize, Queryable, Identifiable, AsChangeset, ToSchema)]
#[diesel(table_name = r09_transmission_locations_raw)]
pub struct TransmissionLocationRaw {
    /// Primary key
    pub id: i64,
    /// ID of the region where telegram was transmitted
    pub region: i64,
    /// Reporting Point inside the r09 telegram (*meldepunkt*) ID
    pub reporting_point: i32,
    /// Report location latitude
    pub lat: f64,
    /// Report location longitude
    pub lon: f64,
    /// Trekkie run from which this undeduped location was inferred
    pub trekkie_run: uuid::Uuid,
    /// User, from whose trekkie run this undeduped location was inferred
    pub run_owner: uuid::Uuid,
}

/// This struct inserts into the table corresponding to [`TransmissionLocationRaw`]
#[derive(Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = r09_transmission_locations_raw)]
pub struct InsertTransmissionLocationRaw {
    /// Primary key. During INSERT should be [`None`] so DB can auto-increment it
    pub id: Option<i64>,
    /// ID of the region where telegram was transmitted
    pub region: i64,
    /// Reporting Point inside the r09 telegram (*meldepunkt*) ID
    pub reporting_point: i32,
    /// Report location latitude
    pub lat: f64,
    /// Report location longitude
    pub lon: f64,
    /// Trekkie run from which this undeduped location was inferred
    pub trekkie_run: uuid::Uuid,
    /// User, from whose trekkie run this undeduped location was inferred
    pub run_owner: uuid::Uuid,
}
///
/// The transmission location that get sent out as part of [`LocationsJson`] from datacare API
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiTransmissionLocation {
    /// latitude
    pub lat: f64,
    /// longitude
    pub lon: f64,
    /// any extra information, as long as it is valid JSON
    pub properties: serde_json::Value,
}

/// The format used by datacare API to send transmission locations out for a given region
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LocationsJson {
    /// The region for which the locations returned
    pub region: crate::locations::region::Region,
    /// Hasmap of a transmission location to a region
    pub transmission_locations: HashMap<i64, ApiTransmissionLocation>,
}

/// Error for associated functions and methods over [`TransmissionLocation`] struct
pub enum TransmissionLocaionError {
    /// Input provided was empty
    EmptyInput,
    /// Expected the data to be from the same region, got from several instead.
    RegionMismatch,
    /// Expected the data for specific [`TransmissionLocation.reporting_point`], got from a
    /// different one instead
    ReportingPointMismatch,
    /// Filtering produced no valid matches
    NoMatches,
}

impl ApiTransmissionLocation {
    /// Updates property field with epsg3857 coordinates, calculated from `loc` and `lon` fileds of
    /// the struct. If field doesn't exist, creates it.
    pub fn update_epsg3857(&mut self) {
        // Convert the coords to pseudo-mercator (epsg3857)
        const EARTH_RADIUS_M: f64 = 6_378_137_f64;
        let x = EARTH_RADIUS_M * self.lon.to_radians();
        let y =
            ((self.lat.to_radians() / 2. + std::f64::consts::PI / 4.).tan()).ln() * EARTH_RADIUS_M;

        // serialize value
        if let Ok(epsg_val) = serde_json::from_str(&format!("{{ \"x\":{x}, \"y\":{y} }}")) {
            self.properties["epsg3857"] = epsg_val;
        } else {
            eprintln!(
                "epsg3857 property update skipped: Could not serialize {x} and {y} into json Value!"
            );
        }
    }
}

type TransmissionLocationResult = Result<InsertTransmissionLocation, TransmissionLocaionError>;
impl InsertTransmissionLocation {
    /// Maximum distance at which the raw point is considered to be corresponding to the report
    /// location cluster
    // TODO: Should we make this configurable?
    pub const MAX_SANE_DISTANCE: f64 = 50_f64;

    /// This function filters out outliers for specific reporting_point
    fn filter_outliers(
        input: Vec<TransmissionLocationRaw>,
    ) -> Result<Vec<TransmissionLocationRaw>, TransmissionLocaionError> {
        // Edge case
        if input.is_empty() {
            return Err(TransmissionLocaionError::EmptyInput);
        }

        let (lats, lons): (Vec<f64>, Vec<f64>) = input.iter().map(|s| (s.lat, s.lon)).unzip();

        let center_point: (f64, f64) = (
            lats.iter().sum::<f64>() / (lats.len() as f64),
            lons.iter().sum::<f64>() / (lons.len() as f64),
        );

        let mut filtered: Vec<TransmissionLocationRaw> = Vec::new();

        for i in input {
            if i.distance_from(center_point) < SANE_INTERPOLATION_DISTANCE as f64 {
                filtered.push(i)
            }
        }

        if filtered.is_empty() {
            return Err(TransmissionLocaionError::NoMatches);
        }

        Ok(filtered)
    }

    /// This function creates the [`InsertTransmissionLocation`] from the vector of raw
    /// transmission locations. Points are averaged, any outliers further away then
    /// [`MAX_SANE_DISTANCE`] are discarded. All the [`TransmissionLocationRaw`] should have the
    /// same region, or the function will fail.
    ///
    /// **This is default way for updating the [`TransmissionLocation`]**. The analysis should be
    /// performed on the whole set of raw locations, to prevent biasing the data.
    pub fn try_from_raw(raw: Vec<TransmissionLocationRaw>) -> TransmissionLocationResult {
        if raw.is_empty() {
            return Err(TransmissionLocaionError::EmptyInput);
        }
        let region = raw[0].region;
        let reporting_point = raw[0].reporting_point;
        for loc in &raw {
            if loc.region != region {
                return Err(TransmissionLocaionError::RegionMismatch);
            }
            if loc.reporting_point != reporting_point {
                return Err(TransmissionLocaionError::ReportingPointMismatch);
            }
        }

        let filtered = Self::filter_outliers(raw)?;

        let (lats, lons): (Vec<f64>, Vec<f64>) = filtered.iter().map(|v| (v.lat, v.lon)).unzip();
        let (lat, lon): (f64, f64) = (
            lats.iter().sum::<f64>() / (lats.len() as f64),
            lons.iter().sum::<f64>() / (lons.len() as f64),
        );

        Ok(InsertTransmissionLocation {
            id: None,
            region,
            reporting_point,
            lat,
            lon,
            ground_truth: false,
        })
    }
}

/// This trait calculates distance between two objects containing positional (latitude, longitude)
/// data.
pub trait DistanceFrom<T> {
    /// This function returns distance in meters between two objects
    fn distance_from(&self, other: T) -> f64;
}

// The impl that does all the heavy lifting
impl DistanceFrom<(f64, f64)> for (f64, f64) {
    fn distance_from(&self, other: (f64, f64)) -> f64 {
        let (self_lat, self_lon) = (self.0.to_radians(), self.1.to_radians());
        let (other_lat, other_lon) = (other.0.to_radians(), other.1.to_radians());

        let a = (((other_lat - self_lat) / 2_f64).sin()).powi(2)
            + self_lat.cos() * other_lat.cos() * (((other_lon - self_lon) / 2_f64).sin()).powi(2);
        let c = 2_f64 * a.sqrt().atan2((1_f64 - a).sqrt());

        MEAN_EARTH_RADIUS as f64 * c
    }
}

impl DistanceFrom<(f64, f64)> for TransmissionLocationRaw {
    fn distance_from(&self, other: (f64, f64)) -> f64 {
        let lat = self.lat;
        let lon = self.lon;

        (lat, lon).distance_from(other)
    }
}
