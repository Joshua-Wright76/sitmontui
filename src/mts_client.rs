use std::time::SystemTime;

use anyhow::Result;
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::data::DataProvider;
use crate::model::{
    parse_timestamp, MapObject, ObjectKind, ObjectMetadata, Severity, ShipStatus, Signal, Warship,
    WorldLeader,
};

const MTS_BASE_URL: &str = "https://monitor-the-situation.com/api";

#[derive(Debug, Clone, Deserialize)]
struct MtsEvent {
    id: String,
    title: String,
    #[serde(default)]
    summary: String,
    category: String,
    #[serde(default)]
    subtype: String,
    severity: i32,
    lat: f64,
    lng: f64,
    #[serde(default)]
    location_name: String,
    #[serde(default)]
    country: String,
    #[serde(default)]
    region: String,
    signal_count: i32,
    confidence: i32,
    is_active: bool,
    #[serde(default)]
    source_types: String,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct MtsAircraft {
    icao: String,
    #[serde(default)]
    callsign: String,
    lat: f64,
    lng: f64,
    altitude: i32,
    heading: i32,
    speed: i32,
    #[serde(default)]
    #[allow(dead_code)]
    vertical_rate: i32,
    #[serde(default)]
    #[allow(dead_code)]
    squawk: String,
    #[serde(rename = "aircraft_type")]
    ac_type: String,
    #[serde(rename = "aircraft_desc")]
    ac_desc: String,
    #[serde(default)]
    registration: String,
    #[serde(default)]
    country: String,
    on_ground: bool,
    #[allow(dead_code)]
    last_update: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MtsWarship {
    id: String,
    #[serde(rename = "ship_name")]
    name: String,
    #[serde(rename = "ship_type")]
    ship_type: String,
    #[serde(default)]
    hull_number: Option<String>,
    region: String,
    lat: f64,
    lng: f64,
    country: String,
    status: String,
    #[serde(default)]
    group_name: Option<String>,
    #[serde(default)]
    group_type: Option<String>,
    #[serde(default)]
    flagship: i32,
    #[serde(default)]
    source_url: Option<String>,
    #[serde(default)]
    source_date: Option<String>,
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MtsWorldLeader {
    #[serde(rename = "leader_id")]
    id: String,
    #[serde(rename = "leader_name")]
    name: String,
    title: String,
    #[serde(rename = "country_code")]
    country_code: String,
    #[serde(rename = "location_name")]
    location_name: String,
    lat: f64,
    lng: f64,
    activity: String,
    #[serde(default)]
    next_activity: String,
    #[serde(rename = "source_summary")]
    source_summary: String,
    confidence: String,
    updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MtsSignal {
    id: String,
    #[serde(rename = "source_type")]
    source_type: String,
    content: String,
    timestamp: String,
}

pub struct MtsProvider {
    client: Client,
}

impl MtsProvider {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self { client })
    }

    fn fetch_events(&self) -> Result<Vec<MapObject>> {
        let url = format!("{}/events", MTS_BASE_URL);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "MTS API returned status: {}",
                response.status()
            ));
        }

        let mts_events: Vec<MtsEvent> = response.json()?;

        let objects = mts_events
            .into_iter()
            .map(|e| {
                let title = format_title(&e);
                MapObject {
                    id: e.id.clone(),
                    label: title,
                    kind: ObjectKind::Incident,
                    severity: Some(map_severity(e.severity)),
                    ship_status: None,
                    lat: e.lat,
                    lng: e.lng,
                    timestamp: parse_timestamp(&e.created_at),
                    metadata: ObjectMetadata {
                        summary: Some(e.summary),
                        category: Some(e.category),
                        subtype: Some(e.subtype),
                        location: Some(e.location_name),
                        country: Some(e.country),
                        region: Some(e.region),
                        signal_count: Some(e.signal_count),
                        confidence: Some(e.confidence),
                        source_types: Some(e.source_types),
                        created_at: Some(e.created_at),
                        updated_at: Some(e.updated_at),
                        is_active: Some(e.is_active),
                        ..Default::default()
                    },
                }
            })
            .collect();

        Ok(objects)
    }

    fn fetch_aircraft(&self) -> Result<Vec<MapObject>> {
        let url = format!("{}/aircraft", MTS_BASE_URL);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "MTS Aircraft API returned status: {}",
                response.status()
            ));
        }

        let aircraft: Vec<MtsAircraft> = response.json()?;

        let objects = aircraft
            .into_iter()
            .map(|ac| {
                let label = if ac.callsign.trim().is_empty() {
                    format!("{} {}", ac.ac_type.to_uppercase(), ac.registration)
                } else {
                    format!("{} | {}", ac.callsign.to_uppercase(), ac.ac_desc)
                };
                MapObject {
                    id: ac.icao.clone(),
                    label,
                    kind: ObjectKind::Aircraft,
                    severity: None,
                    ship_status: Some(if ac.on_ground {
                        ShipStatus::Anchored
                    } else {
                        ShipStatus::Underway
                    }),
                    lat: ac.lat,
                    lng: ac.lng,
                    timestamp: None, // Aircraft don't have created_at from API
                    metadata: ObjectMetadata {
                        altitude: Some(ac.altitude),
                        heading: Some(ac.heading),
                        speed: Some(ac.speed),
                        aircraft_type: Some(ac.ac_type),
                        callsign: Some(ac.callsign),
                        country: Some(ac.country),
                        ..Default::default()
                    },
                }
            })
            .collect();

        Ok(objects)
    }

    fn fetch_warships(&self) -> Result<Vec<Warship>> {
        let url = format!("{}/fleet", MTS_BASE_URL);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "MTS Fleet API returned status: {}",
                response.status()
            ));
        }

        let ships: Vec<MtsWarship> = response.json()?;

        let warships = ships
            .into_iter()
            .map(|s| Warship {
                id: s.id,
                name: s.name,
                ship_type: s.ship_type,
                hull_number: s.hull_number,
                region: s.region,
                lat: s.lat,
                lng: s.lng,
                country: s.country,
                status: s.status,
                group_name: s.group_name,
                group_type: s.group_type,
                flagship: s.flagship == 1,
                source_url: s.source_url,
                source_date: s.source_date,
                updated_at: s.updated_at,
            })
            .collect();

        Ok(warships)
    }

    fn fetch_leaders(&self) -> Result<Vec<WorldLeader>> {
        let url = format!("{}/vip", MTS_BASE_URL);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "MTS VIP API returned status: {}",
                response.status()
            ));
        }

        let leaders: Vec<MtsWorldLeader> = response.json()?;

        let world_leaders = leaders
            .into_iter()
            .map(|l| WorldLeader {
                id: l.id,
                name: l.name,
                title: l.title,
                country_code: l.country_code,
                location_name: l.location_name,
                lat: l.lat,
                lng: l.lng,
                activity: l.activity,
                next_activity: if l.next_activity.is_empty() {
                    None
                } else {
                    Some(l.next_activity)
                },
                source_summary: l.source_summary,
                confidence: l.confidence,
                updated_at: l.updated_at,
            })
            .collect();

        Ok(world_leaders)
    }

    fn fetch_signals_impl(&self, event_id: &str) -> Result<Vec<Signal>> {
        let url = format!("{}/events/{}/signals", MTS_BASE_URL, event_id);
        let response = self.client.get(&url).send()?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "MTS Signals API returned status: {}",
                response.status()
            ));
        }

        let mts_signals: Vec<MtsSignal> = response.json()?;

        let signals = mts_signals
            .into_iter()
            .map(|s| Signal {
                id: s.id,
                source_type: s.source_type,
                content: s.content,
                timestamp: s.timestamp,
            })
            .collect();

        Ok(signals)
    }
}

impl DataProvider for MtsProvider {
    fn fetch_snapshot(&self) -> Result<crate::data::Snapshot> {
        let events = self.fetch_events().unwrap_or_default();
        let aircraft = self.fetch_aircraft().unwrap_or_default();
        let warships = self.fetch_warships().unwrap_or_default();
        let leaders = self.fetch_leaders().unwrap_or_default();

        if events.is_empty() && aircraft.is_empty() && warships.is_empty() && leaders.is_empty() {
            return Err(anyhow::anyhow!("MTS provider returned no usable data"));
        }

        Ok(crate::data::Snapshot {
            events,
            aircraft,
            warships,
            leaders,
            fetched_at: SystemTime::now(),
        })
    }

    fn name(&self) -> &'static str {
        "mts"
    }

    fn fetch_signals(&self, event_id: &str) -> Result<Vec<Signal>> {
        self.fetch_signals_impl(event_id)
    }
}

fn format_title(event: &MtsEvent) -> String {
    let location = if !event.location_name.is_empty() {
        &event.location_name
    } else if !event.country.is_empty() {
        &event.country
    } else {
        &event.region
    };

    if location.is_empty() {
        event.title.clone()
    } else {
        format!("{} ({})", event.title, location)
    }
}

fn map_severity(mts_severity: i32) -> Severity {
    match mts_severity {
        0 => Severity::Low, // Info
        1 => Severity::Low,
        2 => Severity::Medium,
        3 => Severity::High,
        4 => Severity::Critical,
        _ => Severity::Low,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_severity() {
        assert_eq!(map_severity(0), Severity::Low);
        assert_eq!(map_severity(1), Severity::Low);
        assert_eq!(map_severity(2), Severity::Medium);
        assert_eq!(map_severity(3), Severity::High);
        assert_eq!(map_severity(4), Severity::Critical);
    }
}
