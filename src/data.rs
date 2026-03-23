use std::time::SystemTime;

use anyhow::Result;

use crate::model::{
    MapObject, ObjectKind, ObjectMetadata, Severity, ShipStatus, Signal, Warship, WorldLeader,
};

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub events: Vec<MapObject>,
    pub aircraft: Vec<MapObject>,
    pub warships: Vec<Warship>,
    pub leaders: Vec<WorldLeader>,
    pub fetched_at: SystemTime,
}

pub trait DataProvider {
    fn fetch_snapshot(&self) -> Result<Snapshot>;
    fn name(&self) -> &'static str;
    fn fetch_signals(&self, _event_id: &str) -> Result<Vec<Signal>> {
        Ok(Vec::new())
    }
}

pub struct MockProvider;

impl DataProvider for MockProvider {
    fn fetch_snapshot(&self) -> Result<Snapshot> {
        Ok(Snapshot {
            events: Vec::new(),
            aircraft: Vec::new(),
            warships: Vec::new(),
            leaders: Vec::new(),
            fetched_at: SystemTime::now(),
        })
    }

    fn name(&self) -> &'static str {
        "mock"
    }
}

/// Provider that loads data from test fixtures (frozen MTS dataset)
pub struct FixtureProvider {
    events_path: String,
    aircraft_path: String,
}

impl FixtureProvider {
    pub fn from_default_fixtures() -> Self {
        Self {
            events_path: "tests/fixtures/events.json".to_string(),
            aircraft_path: "tests/fixtures/aircraft.json".to_string(),
        }
    }

    #[allow(dead_code)]
    pub fn with_paths(events: String, aircraft: String) -> Self {
        Self {
            events_path: events,
            aircraft_path: aircraft,
        }
    }

    fn load_events(&self) -> Result<Vec<MapObject>> {
        let content = std::fs::read_to_string(&self.events_path)?;
        let events: Vec<serde_json::Value> = serde_json::from_str(&content)?;

        let mut objects = Vec::new();
        for event in events {
            let id = event["id"].as_str().unwrap_or("unknown").to_string();
            let title = event["title"].as_str().unwrap_or("Untitled").to_string();
            let lat = event["lat"].as_f64().unwrap_or(0.0);
            let lng = event["lng"].as_f64().unwrap_or(0.0);
            let severity = match event["severity"].as_i64() {
                Some(4) => Severity::Critical,
                Some(3) => Severity::High,
                Some(2) => Severity::Medium,
                _ => Severity::Low,
            };
            let created_at = event["created_at"].as_str().map(|s| s.to_string());

            objects.push(MapObject {
                id,
                label: title,
                kind: ObjectKind::Incident,
                severity: Some(severity),
                ship_status: None,
                lat,
                lng,
                timestamp: created_at
                    .as_ref()
                    .and_then(|dt| crate::model::parse_timestamp(dt)),
                metadata: ObjectMetadata {
                    summary: event["summary"].as_str().map(|s| s.to_string()),
                    category: event["category"].as_str().map(|s| s.to_string()),
                    subtype: event["subtype"].as_str().map(|s| s.to_string()),
                    location: event["location_name"].as_str().map(|s| s.to_string()),
                    country: event["country"].as_str().map(|s| s.to_string()),
                    region: event["region"].as_str().map(|s| s.to_string()),
                    signal_count: event["signal_count"].as_i64().map(|n| n as i32),
                    confidence: event["confidence"].as_i64().map(|n| n as i32),
                    source_types: event["source_types"].as_str().map(|s| s.to_string()),
                    created_at,
                    updated_at: event["updated_at"].as_str().map(|s| s.to_string()),
                    ..Default::default()
                },
            });
        }

        Ok(objects)
    }

    fn load_aircraft(&self) -> Result<Vec<MapObject>> {
        let content = std::fs::read_to_string(&self.aircraft_path)?;
        let aircraft: Vec<serde_json::Value> = serde_json::from_str(&content)?;

        let mut objects = Vec::new();
        for ac in aircraft {
            let icao = ac["icao"].as_str().unwrap_or("unknown").to_string();
            let callsign = ac["callsign"].as_str().unwrap_or("").to_string();
            let lat = ac["lat"].as_f64().unwrap_or(0.0);
            let lng = ac["lng"].as_f64().unwrap_or(0.0);
            let ac_type = ac["aircraft_type"].as_str().unwrap_or("unknown");
            let ac_desc = ac["aircraft_desc"].as_str().unwrap_or("");
            let on_ground = ac["on_ground"].as_bool().unwrap_or(false);

            let label = if callsign.trim().is_empty() {
                format!("{} {}", ac_type.to_uppercase(), icao)
            } else {
                format!("{} | {}", callsign.to_uppercase(), ac_desc)
            };

            objects.push(MapObject {
                id: icao,
                label,
                kind: ObjectKind::Aircraft,
                severity: None,
                ship_status: Some(if on_ground {
                    ShipStatus::Anchored
                } else {
                    ShipStatus::Underway
                }),
                lat,
                lng,
                timestamp: None,
                metadata: ObjectMetadata {
                    altitude: ac["altitude"].as_i64().map(|n| n as i32),
                    heading: ac["heading"].as_i64().map(|n| n as i32),
                    speed: ac["speed"].as_i64().map(|n| n as i32),
                    aircraft_type: Some(ac_type.to_string()),
                    callsign: Some(callsign),
                    country: ac["country"].as_str().map(|s| s.to_string()),
                    ..Default::default()
                },
            });
        }

        Ok(objects)
    }
}

impl DataProvider for FixtureProvider {
    fn fetch_snapshot(&self) -> Result<Snapshot> {
        let events = self.load_events()?;
        let aircraft = self.load_aircraft()?;

        Ok(Snapshot {
            events,
            aircraft,
            warships: Vec::new(),
            leaders: Vec::new(),
            fetched_at: SystemTime::now(),
        })
    }

    fn name(&self) -> &'static str {
        "fixture"
    }
}

pub fn build_provider_from_env() -> Box<dyn DataProvider> {
    if std::env::var("SITMON_USE_FIXTURES").is_ok() {
        return Box::new(FixtureProvider::from_default_fixtures());
    }

    match crate::mts_client::MtsProvider::new() {
        Ok(provider) => Box::new(provider),
        Err(_) => Box::new(MockProvider),
    }
}
