//! GeoIP service for IP geolocation using MaxMind GeoLite2

use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::sync::Arc;
use tracing::warn;
use utoipa::ToSchema;

/// Structured geolocation result from IP lookup
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    /// ISO 3166-1 alpha-2 country code
    pub country_code: String,
    pub country_name: String,
    pub city: Option<String>,
    pub accuracy_radius_km: u16,
}

/// GeoIP lookup service backed by MaxMind GeoLite2 `.mmdb` database
pub struct GeoIpService {
    reader: Arc<maxminddb::Reader<Vec<u8>>>,
}

/// MaxMind GeoLite2 City record (subset of fields we need)
#[derive(Deserialize, Debug)]
struct GeoLite2City {
    city: Option<CityRecord>,
    country: Option<CountryRecord>,
    location: Option<LocationRecord>,
}

#[derive(Deserialize, Debug)]
struct CityRecord {
    names: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Deserialize, Debug)]
struct CountryRecord {
    iso_code: Option<String>,
    names: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Deserialize, Debug)]
struct LocationRecord {
    latitude: Option<f64>,
    longitude: Option<f64>,
    accuracy_radius: Option<u16>,
}

impl GeoIpService {
    /// Create a new GeoIP service by loading the `.mmdb` file from disk.
    /// Returns `None` if the database cannot be loaded.
    pub fn new(database_path: &str) -> Option<Self> {
        match maxminddb::Reader::open_readfile(database_path) {
            Ok(reader) => {
                tracing::info!("GeoIP database loaded from {}", database_path);
                Some(Self {
                    reader: Arc::new(reader),
                })
            }
            Err(e) => {
                warn!("Failed to load GeoIP database from {}: {}", database_path, e);
                None
            }
        }
    }

    /// Look up geolocation for an IP address string.
    /// Returns `None` for private/loopback IPs or lookup failures.
    pub fn lookup(&self, ip_str: &str) -> Option<GeoLocation> {
        let ip: IpAddr = ip_str.parse().ok()?;

        // Skip private/loopback/link-local addresses
        if !is_global_ip(&ip) {
            return None;
        }

        let record: GeoLite2City = self.reader.lookup(ip).ok()?.decode().ok()??;

        let location = record.location.as_ref()?;
        let latitude = location.latitude?;
        let longitude = location.longitude?;

        let country = record.country.as_ref()?;
        let country_code = country.iso_code.clone()?;
        let country_name = country
            .names
            .as_ref()
            .and_then(|n| n.get("en"))
            .cloned()
            .unwrap_or_else(|| "Unknown".to_string());

        let city = record
            .city
            .as_ref()
            .and_then(|c| c.names.as_ref())
            .and_then(|n| n.get("en"))
            .cloned();

        let accuracy_radius_km = location.accuracy_radius.unwrap_or(0);

        Some(GeoLocation {
            latitude,
            longitude,
            country_code,
            country_name,
            city,
            accuracy_radius_km,
        })
    }
}

/// Check if an IP address is globally routable (not private/loopback/link-local)
fn is_global_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !v4.is_loopback()
                && !v4.is_private()
                && !v4.is_link_local()
                && !v4.is_broadcast()
                && !v4.is_unspecified()
                && !v4.octets().starts_with(&[100, 64]) // CGNAT 100.64.0.0/10
                && !v4.octets().starts_with(&[0])       // 0.0.0.0/8
        }
        IpAddr::V6(v6) => !v6.is_loopback() && !v6.is_unspecified(),
    }
}

/// Calculate the great-circle distance between two points on Earth using
/// the Haversine formula. Returns distance in kilometers.
pub fn haversine_distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();

    let a = (dlat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_KM * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_known_distances() {
        // New York (40.7128, -74.0060) to London (51.5074, -0.1278) ≈ 5570 km
        let dist = haversine_distance_km(40.7128, -74.0060, 51.5074, -0.1278);
        assert!((dist - 5570.0).abs() < 50.0, "NY-London: {dist}");

        // Tokyo (35.6762, 139.6503) to Sydney (-33.8688, 151.2093) ≈ 7823 km
        let dist = haversine_distance_km(35.6762, 139.6503, -33.8688, 151.2093);
        assert!((dist - 7823.0).abs() < 50.0, "Tokyo-Sydney: {dist}");

        // Same point should be 0
        let dist = haversine_distance_km(0.0, 0.0, 0.0, 0.0);
        assert!(dist.abs() < 0.001);

        // Antipodal points ≈ 20015 km (half circumference)
        let dist = haversine_distance_km(0.0, 0.0, 0.0, 180.0);
        assert!((dist - 20015.0).abs() < 100.0, "Antipodal: {dist}");
    }

    #[test]
    fn test_is_global_ip() {
        // Private IPs should not be global
        assert!(!is_global_ip(&"127.0.0.1".parse().unwrap()));
        assert!(!is_global_ip(&"192.168.1.1".parse().unwrap()));
        assert!(!is_global_ip(&"10.0.0.1".parse().unwrap()));
        assert!(!is_global_ip(&"172.16.0.1".parse().unwrap()));
        assert!(!is_global_ip(&"169.254.1.1".parse().unwrap()));

        // Public IPs should be global
        assert!(is_global_ip(&"8.8.8.8".parse().unwrap()));
        assert!(is_global_ip(&"1.1.1.1".parse().unwrap()));
        assert!(is_global_ip(&"203.0.113.1".parse().unwrap()));
    }

    #[test]
    fn test_geolocation_struct_serialization() {
        let geo = GeoLocation {
            latitude: 40.7128,
            longitude: -74.0060,
            country_code: "US".to_string(),
            country_name: "United States".to_string(),
            city: Some("New York".to_string()),
            accuracy_radius_km: 10,
        };
        let json = serde_json::to_string(&geo).unwrap();
        assert!(json.contains("US"));
        assert!(json.contains("40.7128"));
    }
}
