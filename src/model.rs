#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomLevel {
    Regional,
    X2,
    X4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapContrastMode {
    Normal,
    High,
    TransparentSafe,
}

impl MapContrastMode {
    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::High,
            Self::High => Self::TransparentSafe,
            Self::TransparentSafe => Self::Normal,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::High => "high",
            Self::TransparentSafe => "transparent-safe",
        }
    }
}

impl ZoomLevel {
    pub fn factor(self) -> f64 {
        match self {
            Self::Regional => 1.0,
            Self::X2 => 2.0,
            Self::X4 => 4.0,
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Regional => Self::X2,
            Self::X2 => Self::X4,
            Self::X4 => Self::X4,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Regional => Self::Regional,
            Self::X2 => Self::Regional,
            Self::X4 => Self::X2,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Regional => "regional",
            Self::X2 => "2x",
            Self::X4 => "4x",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    #[allow(dead_code)]
    pub fn symbol(self) -> char {
        match self {
            Self::Low => '·',
            Self::Medium => '!',
            Self::High => '▲',
            Self::Critical => '◆',
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Critical => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShipStatus {
    Underway,
    Anchored,
}

impl ShipStatus {
    #[allow(dead_code)]
    pub fn symbol(self) -> char {
        match self {
            Self::Underway => '⛴',
            Self::Anchored => '◇',
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Incident {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Ship {
    pub id: String,
    pub name: String,
    pub status: ShipStatus,
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lng: f64,
    pub max_lng: f64,
}

impl Bounds {
    pub const fn new(min_lat: f64, max_lat: f64, min_lng: f64, max_lng: f64) -> Self {
        Self {
            min_lat,
            max_lat,
            min_lng,
            max_lng,
        }
    }

    pub fn width_lng(self) -> f64 {
        self.max_lng - self.min_lng
    }

    pub fn height_lat(self) -> f64 {
        self.max_lat - self.min_lat
    }

    pub fn center(self) -> (f64, f64) {
        (
            (self.min_lat + self.max_lat) / 2.0,
            (self.min_lng + self.max_lng) / 2.0,
        )
    }

    pub fn contains(self, lat: f64, lng: f64) -> bool {
        lat >= self.min_lat && lat <= self.max_lat && lng >= self.min_lng && lng <= self.max_lng
    }

    pub fn zoomed(self, factor: f64, center: Option<(f64, f64)>) -> Self {
        if factor <= 1.0 {
            return self;
        }
        let (mut c_lat, mut c_lng) = center.unwrap_or_else(|| self.center());
        c_lat = c_lat.clamp(self.min_lat, self.max_lat);
        c_lng = c_lng.clamp(self.min_lng, self.max_lng);

        let half_h = self.height_lat() / (2.0 * factor);
        let half_w = self.width_lng() / (2.0 * factor);

        let min_lat = (c_lat - half_h).max(self.min_lat);
        let max_lat = (c_lat + half_h).min(self.max_lat);
        let min_lng = (c_lng - half_w).max(self.min_lng);
        let max_lng = (c_lng + half_w).min(self.max_lng);

        Self::new(min_lat, max_lat, min_lng, max_lng)
    }

    pub fn clamp_center(self, lat: f64, lng: f64, zoom_factor: f64) -> (f64, f64) {
        if zoom_factor <= 1.0 {
            return (
                lat.clamp(self.min_lat, self.max_lat),
                lng.clamp(self.min_lng, self.max_lng),
            );
        }

        let half_h = self.height_lat() / (2.0 * zoom_factor);
        let half_w = self.width_lng() / (2.0 * zoom_factor);

        (
            lat.clamp(self.min_lat + half_h, self.max_lat - half_h),
            lng.clamp(self.min_lng + half_w, self.max_lng - half_w),
        )
    }
}

#[derive(Debug, Clone)]
pub struct RegionPreset {
    pub name: &'static str,
    pub bounds: Bounds,
    pub projection: ProjectionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionKind {
    Equirectangular,
    Mercator,
    NorthPolar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    Incident,
    Ship,
    Aircraft,
}

#[derive(Debug, Clone)]
pub struct MapObject {
    pub id: String,
    pub label: String,
    pub kind: ObjectKind,
    pub severity: Option<Severity>,
    pub ship_status: Option<ShipStatus>,
    pub lat: f64,
    pub lng: f64,
    pub metadata: ObjectMetadata,
    pub timestamp: Option<i64>, // Unix timestamp for sorting
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub id: String,
    pub source_type: String,
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Default)]
pub struct ObjectMetadata {
    pub summary: Option<String>,
    pub category: Option<String>,
    pub subtype: Option<String>,
    pub location: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub signal_count: Option<i32>,
    pub confidence: Option<i32>,
    pub source_types: Option<String>,
    pub created_at: Option<String>,
    #[allow(dead_code)]
    pub updated_at: Option<String>,
    pub altitude: Option<i32>,
    pub heading: Option<i32>,
    pub speed: Option<i32>,
    pub aircraft_type: Option<String>,
    pub callsign: Option<String>,
    pub signals: Option<Vec<Signal>>,
}

impl MapObject {
    pub fn weight(&self) -> u8 {
        match self.kind {
            ObjectKind::Incident => match self.severity {
                Some(Severity::Critical) => 4,
                Some(Severity::High) => 3,
                Some(Severity::Medium) => 2,
                _ => 1,
            },
            ObjectKind::Ship => 1,
            ObjectKind::Aircraft => 2,
        }
    }

    pub fn matches_search(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.label.to_lowercase().contains(&q)
            || self
                .metadata
                .summary
                .as_ref()
                .is_some_and(|s| s.to_lowercase().contains(&q))
            || self
                .metadata
                .category
                .as_ref()
                .is_some_and(|s| s.to_lowercase().contains(&q))
            || self
                .metadata
                .location
                .as_ref()
                .is_some_and(|s| s.to_lowercase().contains(&q))
            || self
                .metadata
                .country
                .as_ref()
                .is_some_and(|s| s.to_lowercase().contains(&q))
    }
}

impl Warship {
    pub fn matches_search(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q)
            || self.ship_type.to_lowercase().contains(&q)
            || self.country.to_lowercase().contains(&q)
            || self.region.to_lowercase().contains(&q)
    }
}

impl WorldLeader {
    pub fn matches_search(&self, query: &str) -> bool {
        let q = query.to_lowercase();
        self.name.to_lowercase().contains(&q)
            || self.title.to_lowercase().contains(&q)
            || self.location_name.to_lowercase().contains(&q)
            || self.activity.to_lowercase().contains(&q)
    }
}

#[derive(Debug, Clone)]
pub struct Warship {
    pub id: String,
    pub name: String,
    pub ship_type: String,
    pub hull_number: Option<String>,
    pub region: String,
    pub lat: f64,
    pub lng: f64,
    pub country: String,
    pub status: String,
    pub group_name: Option<String>,
    pub group_type: Option<String>,
    pub flagship: bool,
    pub source_url: Option<String>,
    pub source_date: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct WorldLeader {
    pub id: String,
    pub name: String,
    pub title: String,
    pub country_code: String,
    pub location_name: String,
    pub lat: f64,
    pub lng: f64,
    pub activity: String,
    pub next_activity: Option<String>,
    pub source_summary: String,
    pub confidence: String,
    pub updated_at: String,
}

/// Parse datetime string "2026-03-16 22:05:36" into Unix timestamp
pub fn parse_timestamp(datetime: &str) -> Option<i64> {
    let parts: Vec<&str> = datetime.split_whitespace().collect();
    if parts.len() != 2 {
        return None;
    }

    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();

    if date_parts.len() != 3 || time_parts.len() != 3 {
        return None;
    }

    let year = date_parts[0].parse::<i32>().ok()?;
    let month = date_parts[1].parse::<u32>().ok()?;
    let day = date_parts[2].parse::<u32>().ok()?;
    let hour = time_parts[0].parse::<u32>().ok()?;
    let minute = time_parts[1].parse::<u32>().ok()?;
    let second = time_parts[2].parse::<u32>().ok()?;

    Some(datetime_to_seconds(year, month, day, hour, minute, second))
}

fn datetime_to_seconds(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> i64 {
    let days = days_from_ymd(year, month, day);
    (days as i64) * 86400 + (hour as i64) * 3600 + (minute as i64) * 60 + (second as i64)
}

fn days_from_ymd(year: i32, month: u32, day: u32) -> i32 {
    let y = year - 1;
    let mut days = y * 365 + y / 4 - y / 100 + y / 400;
    let month_days = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    days += month_days[(month - 1) as usize] + (day as i32) - 1;
    if month > 2 && is_leap_year(year) {
        days += 1;
    }
    days - 719162 // Days from 1970-01-01
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
