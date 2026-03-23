# Monitor the Situation - Public API Documentation

## Base URL
```
https://monitor-the-situation.com/api
```

## Available Endpoints

### 1. Events (Incidents)
```
GET /api/events
```
Returns live global events with severity ratings.

**Response Format:**
```json
[
  {
    "id": "22109316-b1e2-484a-b5d6-09fa278125b9",
    "title": "Iraqi Resistance Launches Kamikaze Drones...",
    "summary": "Iraqi Resistance fighters targeted Prince Hussein Air Base...",
    "category": "conflict",
    "subtype": "drone_strike",
    "severity": 3,
    "lat": 31.1781,
    "lng": 37.4377,
    "location_name": "Prince Hussein Air Base, Jordan",
    "country": "Jordan",
    "region": "Middle East",
    "signal_count": 1,
    "confidence": 60,
    "is_active": true,
    "source_types": "twitter",
    "created_at": "2026-03-16 22:05:36",
    "updated_at": "2026-03-16 22:05:37"
  }
]
```

**Severity Levels:**
- 0: Info
- 1: Low
- 2: Medium
- 3: High
- 4: Critical

**Categories:**
- conflict
- disaster
- political
- (others available)

### 2. Aircraft (ADS-B Tracking)
```
GET /api/aircraft
```
Returns live military and civilian aircraft positions.

**Response Format:**
```json
[
  {
    "icao": "ae2fd0",
    "callsign": "TROY701",
    "lat": 8.96375,
    "lng": -79.60111,
    "altitude": 1700,
    "heading": 358,
    "speed": 93,
    "vertical_rate": 832,
    "squawk": "0",
    "aircraft_type": "military",
    "aircraft_desc": "Q9",
    "registration": "CBP-113",
    "country": "US",
    "on_ground": false,
    "last_update": "2026-03-16 22:05:40"
  }
]
```

### 3. Ships (AIS Tracking)
```
GET /api/ships
```
**Note:** Requires viewport parameters (zoom level). Returns error if viewport too large.

### 4. Markets (Prediction Markets)
```
GET /api/markets
```
Kalshi prediction market data

### 5. Weather
```
GET /api/weather
```
Weather radar and satellite data

### 6. Infrastructure Data
```
GET /api/bases           # Military bases
GET /api/cables          # Submarine cables
GET /api/cable-landings  # Cable landing points
GET /api/pipelines       # Pipeline routes
GET /api/power-plants    # Power plant locations
GET /api/mines           # Mining operations
GET /api/fleet           # Naval fleet positions
```

### 7. Internet Monitoring
```
GET /api/internet        # Internet outage/disruption data
GET /api/countries       # Country boundaries/geo data
```

### 8. Wildfire Data
```
GET /api/fires
```
Active wildfire locations

### 9. Feed Status
```
GET /api/feeds/status/v2
```
Status of news feed ingestion

### 10. Pizza Index 🍕
```
GET /api/pizza
```
Pentagon Pizza Index (food delivery monitoring)

### 11. Event Signals
```
GET /api/events/{event_id}/signals
```
Raw signal data for a specific event

## Key Features

✅ **No API key required** - Completely public  
✅ **Real-time updates** - Live data streaming  
✅ **CORS enabled** - Accessible from any origin  
✅ **JSON format** - Standard REST API  
✅ **Rich data** - Events, aircraft, ships, markets, infrastructure

## Rate Limiting

No explicit rate limiting detected, but be respectful:
- Events: ~220KB response (fetch every 30-60s)
- Aircraft: Smaller response (fetch every 5-10s)
- Ships: Requires zoom parameters

## Integration Example (Rust)

```rust
use reqwest;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Event {
    id: String,
    title: String,
    summary: String,
    category: String,
    severity: i32,
    lat: f64,
    lng: f64,
    country: String,
    region: String,
    is_active: bool,
    source_types: String,
}

async fn fetch_events() -> Result<Vec<Event>, reqwest::Error> {
    let url = "https://monitor-the-situation.com/api/events";
    let response = reqwest::get(url).await?;
    let events = response.json::<Vec<Event>>().await?;
    Ok(events)
}
```

## Data Quality

- **Events:** Aggregated from 40+ news sources + X/Twitter
- **Aircraft:** ADS-B transponder data (military & civilian)
- **Ships:** AIS transponder data
- **Markets:** Kalshi prediction markets
- **All data is real-time and continuously updated**

## Notes

- No authentication required
- No usage limits observed
- Data updates continuously (real-time)
- Returns ~300-500 events at a time
- Aircraft data includes military assets (callsigns like TROY, COBRA, etc.)
- Event severity is automatically calculated (0-4 scale)

---

**Perfect for your TUI app!** You can replace the mock data and ACLED integration with this single API endpoint.
