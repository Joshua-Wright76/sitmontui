use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::data::{DataProvider, Snapshot};
use crate::model::{MapObject, Severity, Signal, Warship, WorldLeader};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    Input,   // Typing query
    Results, // Viewing and navigating results
}

#[derive(Debug, Clone)]
pub enum SearchResult {
    Feed(usize),    // Index in cached_objects
    Warship(usize), // Index in snapshot.warships
    Leader(usize),  // Index in snapshot.leaders
}

impl SearchResult {
    pub fn focus(&self) -> PaneFocus {
        match self {
            SearchResult::Feed(_) => PaneFocus::Feed,
            SearchResult::Warship(_) => PaneFocus::Warships,
            SearchResult::Leader(_) => PaneFocus::Leaders,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneFocus {
    Feed,
    Warships,
    Leaders,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerId {
    Incidents,
    Ships,
}

#[derive(Debug, Clone)]
pub struct LayerState {
    pub id: LayerId,
    pub name: &'static str,
    pub visible: bool,
    pub default_visible: bool,
}

impl LayerState {
    fn new(id: LayerId, name: &'static str, default_visible: bool) -> Self {
        Self {
            id,
            name,
            visible: default_visible,
            default_visible,
        }
    }
}

impl PaneFocus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Feed => "feed",
            Self::Warships => "warships",
            Self::Leaders => "leaders",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Feed => Self::Warships,
            Self::Warships => Self::Leaders,
            Self::Leaders => Self::Feed,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Feed => Self::Leaders,
            Self::Warships => Self::Feed,
            Self::Leaders => Self::Warships,
        }
    }
}

#[derive(Clone)]
pub struct App {
    pub severity_filter: [bool; 4],
    pub layers: Vec<LayerState>,
    pub layer_panel_open: bool,
    pub layer_cursor: usize,
    pub focus: PaneFocus,
    pub selected_idx: usize,
    pub selected_idx_warships: usize,
    pub selected_idx_leaders: usize,
    pub expanded_idx: Option<usize>,
    pub status: String,
    pub snapshot: Snapshot,
    pub last_refresh: SystemTime,
    pub next_refresh: Instant,
    pub tick_rate: Duration,
    pub quit: bool,
    // Cached visible objects to avoid recalculation
    cached_objects: Vec<MapObject>,
    cache_dirty: bool,
    // Thread-safe cache for signals fetched when expanding events
    signals_cache: Arc<Mutex<HashMap<String, Vec<Signal>>>>,
    // Event ID waiting for signal fetch
    pending_signal_fetch: Option<String>,
    // Search panel state
    pub is_searching: bool,
    pub search_mode: SearchMode,
    pub search_query: String,
    pub search_selected_idx: usize,
    pub search_results: Vec<SearchResult>,
}

impl App {
    pub fn new(_regions: Vec<()>, provider: &dyn DataProvider) -> Self {
        let snapshot = provider.fetch_snapshot().unwrap_or_else(|_| Snapshot {
            events: Vec::new(),
            aircraft: Vec::new(),
            warships: Vec::new(),
            leaders: Vec::new(),
            fetched_at: SystemTime::now(),
        });
        let now = Instant::now();

        let mut app = Self {
            severity_filter: [true, true, true, true],
            layers: vec![
                LayerState::new(LayerId::Incidents, "Incidents", true),
                LayerState::new(LayerId::Ships, "Ships", true),
            ],
            layer_panel_open: false,
            layer_cursor: 0,
            focus: PaneFocus::Feed,
            selected_idx: 0,
            selected_idx_warships: 0,
            selected_idx_leaders: 0,
            expanded_idx: None,
            status: format!("ready ({})", provider.name()),
            last_refresh: snapshot.fetched_at,
            snapshot,
            next_refresh: now + Duration::from_secs(30),
            tick_rate: Duration::from_millis(100),
            quit: false,
            cached_objects: Vec::new(),
            cache_dirty: true,
            signals_cache: Arc::new(Mutex::new(HashMap::new())),
            pending_signal_fetch: None,
            is_searching: false,
            search_mode: SearchMode::Input,
            search_query: String::new(),
            search_selected_idx: 0,
            search_results: Vec::new(),
        };

        // Initial cache build
        app.rebuild_cache();
        app
    }

    /// Rebuild the cached visible objects
    fn rebuild_cache(&mut self) {
        let mut out = Vec::new();

        if self.layer_visible(LayerId::Incidents) {
            for event in &self.snapshot.events {
                if let Some(severity) = event.severity {
                    if self.severity_filter[severity.index()] {
                        out.push(event.clone());
                    }
                }
            }
        }

        if self.layer_visible(LayerId::Ships) {
            for aircraft in &self.snapshot.aircraft {
                out.push(aircraft.clone());
            }
        }

        // Sort by timestamp (most recent first), then by weight for items without timestamps
        out.sort_by(|a, b| {
            // Compare timestamps first (most recent = largest timestamp first)
            match (b.timestamp, a.timestamp) {
                (Some(b_ts), Some(a_ts)) => b_ts.cmp(&a_ts),
                (Some(_), None) => std::cmp::Ordering::Less, // b has timestamp, a doesn't -> b comes first
                (None, Some(_)) => std::cmp::Ordering::Greater, // a has timestamp, b doesn't -> a comes first
                (None, None) => {
                    // Neither has timestamp, fall back to weight then label
                    b.weight()
                        .cmp(&a.weight())
                        .then_with(|| a.label.cmp(&b.label))
                }
            }
        });

        self.cached_objects = out;
        self.cache_dirty = false;
    }

    /// Mark cache as dirty (needs rebuild)
    fn invalidate_cache(&mut self) {
        self.cache_dirty = true;
    }

    /// Compute feed item heights for a given width.
    /// Returns the heights for each visible object.
    pub fn compute_feed_heights(&self, width: u16) -> Vec<usize> {
        let title_width = width.saturating_sub(7) as usize;
        self.cached_objects
            .iter()
            .map(|obj| {
                let title_len = obj.label.len();
                let title_lines = if title_width > 0 {
                    (title_len + title_width.saturating_sub(1)) / title_width
                } else {
                    1
                };
                title_lines.min(2) + 2
            })
            .collect()
    }

    /// Get visible objects (uses cache if clean)
    pub fn visible_objects(&self) -> &Vec<MapObject> {
        &self.cached_objects
    }

    /// Get visible objects mutably (rebuilds cache if dirty)
    pub fn visible_objects_mut(&mut self) -> &Vec<MapObject> {
        if self.cache_dirty {
            self.rebuild_cache();
        }
        &self.cached_objects
    }

    pub fn selected_object(&self) -> Option<&MapObject> {
        let objects = self.visible_objects();
        if objects.is_empty() {
            return None;
        }
        let idx = self.selected_idx.min(objects.len() - 1);
        objects.get(idx)
    }

    pub fn get_signals_for_event(&self, event_id: &str) -> Option<Vec<Signal>> {
        self.signals_cache.lock().ok()?.get(event_id).cloned()
    }

    pub fn current_selection(&self) -> (usize, usize) {
        match self.focus {
            PaneFocus::Feed => (self.selected_idx, self.visible_objects().len()),
            PaneFocus::Warships => (self.selected_idx_warships, self.filtered_warships().len()),
            PaneFocus::Leaders => (self.selected_idx_leaders, self.filtered_leaders().len()),
        }
    }

    pub fn set_current_selection(&mut self, idx: usize) {
        match self.focus {
            PaneFocus::Feed => self.selected_idx = idx,
            PaneFocus::Warships => self.selected_idx_warships = idx,
            PaneFocus::Leaders => self.selected_idx_leaders = idx,
        }
    }

    pub fn start_search(&mut self) {
        self.is_searching = true;
        self.search_mode = SearchMode::Input;
        self.search_query.clear();
        self.search_selected_idx = 0;
        self.search_results.clear();
        self.status = String::from("search mode - type query, ENTER to search");
    }

    pub fn exit_search(&mut self) {
        self.is_searching = false;
        self.search_mode = SearchMode::Input;
        self.search_query.clear();
        self.search_selected_idx = 0;
        self.search_results.clear();
        self.status = String::from("search exited");
    }

    pub fn enter_search_results(&mut self) {
        self.search_mode = SearchMode::Results;
        self.search_selected_idx = 0;
        self.build_search_results();
        self.status = String::from("search results - j/k navigate, ENTER select");
    }

    fn build_search_results(&mut self) {
        self.search_results.clear();
        let q = &self.search_query;

        if q.is_empty() {
            return;
        }

        for (idx, event) in self.cached_objects.iter().enumerate() {
            if event.matches_search(q) {
                self.search_results.push(SearchResult::Feed(idx));
            }
        }

        for (idx, warship) in self.snapshot.warships.iter().enumerate() {
            if warship.matches_search(q) {
                self.search_results.push(SearchResult::Warship(idx));
            }
        }

        for (idx, leader) in self.snapshot.leaders.iter().enumerate() {
            if leader.matches_search(q) {
                self.search_results.push(SearchResult::Leader(idx));
            }
        }
    }

    pub fn search_select_next(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        self.search_selected_idx = (self.search_selected_idx + 1) % self.search_results.len();
    }

    pub fn search_select_prev(&mut self) {
        if self.search_results.is_empty() {
            return;
        }
        if self.search_selected_idx == 0 {
            self.search_selected_idx = self.search_results.len() - 1;
        } else {
            self.search_selected_idx -= 1;
        }
    }

    pub fn activate_search_result(&mut self) -> bool {
        if self.search_results.is_empty() {
            return false;
        }

        let result = self.search_results.get(self.search_selected_idx).cloned();
        if let Some(result) = result {
            match result {
                SearchResult::Feed(idx) => {
                    self.focus = PaneFocus::Feed;
                    self.selected_idx = idx;
                }
                SearchResult::Warship(idx) => {
                    self.focus = PaneFocus::Warships;
                    let warship_id = self.snapshot.warships.get(idx).map(|w| w.id.clone());
                    let sorted_warships = self.sorted_warships();
                    let sorted_pos =
                        warship_id.and_then(|id| sorted_warships.iter().position(|w| w.id == id));
                    self.selected_idx_warships = sorted_pos.unwrap_or(idx);
                }
                SearchResult::Leader(idx) => {
                    self.focus = PaneFocus::Leaders;
                    self.selected_idx_leaders = idx;
                }
            }
            self.exit_search();
            return true;
        }
        false
    }

    pub fn clamp_selections(&mut self) {
        let events_len = self.visible_objects().len();
        if self.selected_idx >= events_len && events_len > 0 {
            self.selected_idx = events_len - 1;
        }
        let warships_len = self.filtered_warships().len();
        if self.selected_idx_warships >= warships_len && warships_len > 0 {
            self.selected_idx_warships = warships_len - 1;
        }
        let leaders_len = self.filtered_leaders().len();
        if self.selected_idx_leaders >= leaders_len && leaders_len > 0 {
            self.selected_idx_leaders = leaders_len - 1;
        }
    }

    pub fn filtered_warships(&self) -> Vec<&Warship> {
        if self.search_query.is_empty() {
            self.snapshot.warships.iter().collect()
        } else {
            let q = &self.search_query;
            self.snapshot
                .warships
                .iter()
                .filter(|w| w.matches_search(q))
                .collect()
        }
    }

    pub fn filtered_leaders(&self) -> Vec<&WorldLeader> {
        if self.search_query.is_empty() {
            self.snapshot.leaders.iter().collect()
        } else {
            let q = &self.search_query;
            self.snapshot
                .leaders
                .iter()
                .filter(|l| l.matches_search(q))
                .collect()
        }
    }

    pub fn sorted_warships(&self) -> Vec<&Warship> {
        let mut warships: Vec<_> = self.snapshot.warships.iter().collect();
        warships.sort_by(|a, b| {
            let a_is_carrier = a.ship_type.to_lowercase().contains("carrier");
            let b_is_carrier = b.ship_type.to_lowercase().contains("carrier");
            let a_is_us = a.country == "US" || a.country == "USA";
            let b_is_us = b.country == "US" || b.country == "USA";

            match (a_is_carrier, b_is_carrier) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (true, true) => match (a_is_us, b_is_us) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                },
                _ => a
                    .ship_type
                    .cmp(&b.ship_type)
                    .then_with(|| a.name.cmp(&b.name)),
            }
        });
        warships
    }

    pub fn tick(&mut self, provider: &dyn DataProvider) {
        // Check if we need to fetch signals for an expanded event
        self.process_pending_signal_fetches();

        if Instant::now() >= self.next_refresh {
            match provider.fetch_snapshot() {
                Ok(snapshot) => {
                    self.snapshot = snapshot;
                    self.last_refresh = self.snapshot.fetched_at;
                    self.next_refresh = Instant::now() + Duration::from_secs(30);
                    self.status = format!("snapshot refreshed ({})", provider.name());
                    self.invalidate_cache();
                    // Keep selection valid
                    let count = self.visible_objects_mut().len();
                    if self.selected_idx >= count && count > 0 {
                        self.selected_idx = count - 1;
                    }
                }
                Err(err) => {
                    self.next_refresh = Instant::now() + Duration::from_secs(15);
                    self.status = format!("refresh failed: {err}");
                }
            }
        }
    }

    fn process_pending_signal_fetches(&mut self) {
        if let Some(event_id) = self.pending_signal_fetch.take() {
            let cache = Arc::clone(&self.signals_cache);
            std::thread::spawn(move || {
                if let Ok(provider) = crate::mts_client::MtsProvider::new() {
                    if let Ok(signals) = provider.fetch_signals(&event_id) {
                        if let Ok(mut guard) = cache.lock() {
                            guard.insert(event_id, signals);
                        }
                    }
                }
            });
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent, provider: &dyn DataProvider) {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            self.quit = true;
            return;
        }

        if self.layer_panel_open {
            self.handle_layer_panel_key(key);
            return;
        }

        if self.is_searching {
            self.handle_search_key(key, provider);
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Esc => {
                if self.expanded_idx.is_some() {
                    self.expanded_idx = None;
                    self.status = String::from("detail view closed");
                }
            }
            KeyCode::Char('t') => {
                self.layer_panel_open = true;
                self.status = String::from("layer panel opened");
            }
            KeyCode::Char('1') => self.toggle_severity(Severity::Low),
            KeyCode::Char('2') => self.toggle_severity(Severity::Medium),
            KeyCode::Char('3') => self.toggle_severity(Severity::High),
            KeyCode::Char('4') => self.toggle_severity(Severity::Critical),
            KeyCode::Char('n') => {
                self.select_next();
            }
            KeyCode::Char('p') => {
                self.select_prev();
            }
            KeyCode::Char('g') => match provider.fetch_snapshot() {
                Ok(snapshot) => {
                    self.snapshot = snapshot;
                    self.last_refresh = self.snapshot.fetched_at;
                    self.next_refresh = Instant::now() + Duration::from_secs(30);
                    self.invalidate_cache();
                    self.status = format!("manual refresh ({})", provider.name());
                }
                Err(err) => {
                    self.status = format!("manual refresh failed: {err}");
                }
            },
            KeyCode::Char('k') => self.select_prev(),
            KeyCode::Char('j') => self.select_next(),
            KeyCode::Up => self.select_prev(),
            KeyCode::Down => self.select_next(),
            KeyCode::Enter => {
                self.toggle_expanded(provider);
            }
            KeyCode::Char('h') => {
                self.focus = self.focus.prev();
                self.expanded_idx = None;
                self.status = format!("focus: {}", self.focus.label());
            }
            KeyCode::Char('l') => {
                self.focus = self.focus.next();
                self.expanded_idx = None;
                self.status = format!("focus: {}", self.focus.label());
            }
            KeyCode::Tab => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.focus = self.focus.prev();
                } else {
                    self.focus = self.focus.next();
                }
                self.expanded_idx = None;
                self.status = format!("focus: {}", self.focus.label());
            }
            KeyCode::BackTab => {
                self.focus = self.focus.prev();
                self.expanded_idx = None;
                self.status = format!("focus: {}", self.focus.label());
            }
            KeyCode::Char('/') => {
                self.start_search();
            }
            _ => {}
        }
    }

    fn handle_layer_panel_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Char('t') => {
                self.layer_panel_open = false;
                self.status = String::from("layer panel closed");
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.layer_cursor > 0 {
                    self.layer_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.layer_cursor + 1 < self.layers.len() {
                    self.layer_cursor += 1;
                }
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                let layer_name = self.layers.get(self.layer_cursor).map(|l| l.name);
                if let Some(layer_name) = layer_name {
                    if let Some(layer) = self.layers.get_mut(self.layer_cursor) {
                        layer.visible = !layer.visible;
                        let status = if layer.visible { "on" } else { "off" };
                        self.invalidate_cache();
                        self.status = format!("{}: {}", layer_name.to_lowercase(), status);
                    }
                }
            }
            KeyCode::Char('a') => {
                let all_on = self.layers.iter().all(|l| l.visible);
                for layer in &mut self.layers {
                    layer.visible = !all_on;
                }
                self.invalidate_cache();
                self.status = if all_on {
                    String::from("all layers off")
                } else {
                    String::from("all layers on")
                };
            }
            KeyCode::Char('d') => {
                for layer in &mut self.layers {
                    layer.visible = layer.default_visible;
                }
                self.invalidate_cache();
                self.status = String::from("layers reset to defaults");
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent, provider: &dyn DataProvider) {
        match self.search_mode {
            SearchMode::Input => match key.code {
                KeyCode::Esc => {
                    self.exit_search();
                }
                KeyCode::Enter => {
                    if !self.search_query.is_empty() {
                        self.enter_search_results();
                    }
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    if !self.search_query.is_empty() {
                        self.build_search_results();
                    } else {
                        self.search_results.clear();
                        self.status = String::from("search mode - type query, ENTER to search");
                    }
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.build_search_results();
                }
                _ => {}
            },
            SearchMode::Results => match key.code {
                KeyCode::Esc => {
                    self.search_mode = SearchMode::Input;
                    self.search_selected_idx = 0;
                    if self.search_query.is_empty() {
                        self.exit_search();
                    } else {
                        self.status = String::from("search mode - type query, ENTER to search");
                    }
                }
                KeyCode::Enter => {
                    self.activate_search_result();
                    self.toggle_expanded(provider);
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.search_select_next();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.search_select_prev();
                }
                _ => {}
            },
        }
    }

    pub fn layer_visible(&self, id: LayerId) -> bool {
        self.layers
            .iter()
            .find(|layer| layer.id == id)
            .map(|layer| layer.visible)
            .unwrap_or(false)
    }

    fn select_next(&mut self) {
        let (idx, total) = self.current_selection();
        if total > 0 {
            let new_idx = (idx + 1) % total;
            self.set_current_selection(new_idx);
            self.expanded_idx = None;
        }
    }

    fn select_prev(&mut self) {
        let (idx, total) = self.current_selection();
        if total > 0 {
            let new_idx = if idx == 0 { total - 1 } else { idx - 1 };
            self.set_current_selection(new_idx);
            self.expanded_idx = None;
        }
    }

    fn toggle_expanded(&mut self, _provider: &dyn DataProvider) {
        let (idx, total) = self.current_selection();

        if total == 0 {
            return;
        }

        let idx = idx.min(total - 1);

        if self.expanded_idx == Some(idx) {
            self.expanded_idx = None;
            self.status = String::from("detail view closed");
        } else {
            self.expanded_idx = Some(idx);
            self.status = String::from("detail view opened");

            if self.focus == PaneFocus::Feed {
                let object = self.visible_objects().get(idx);
                if let Some(obj) = object {
                    if obj.kind == crate::model::ObjectKind::Incident {
                        let needs_fetch = self
                            .signals_cache
                            .lock()
                            .map(|guard| !guard.contains_key(&obj.id))
                            .unwrap_or(false);
                        if needs_fetch {
                            self.pending_signal_fetch = Some(obj.id.clone());
                        }
                    }
                }
            } else if self.focus == PaneFocus::Warships {
                let warships = self.sorted_warships();
                let _warship = warships.get(idx);
            } else if self.focus == PaneFocus::Leaders {
                let _leader = self.snapshot.leaders.get(idx);
            }
        }
    }

    fn toggle_severity(&mut self, sev: Severity) {
        let idx = sev.index();
        self.severity_filter[idx] = !self.severity_filter[idx];
        self.invalidate_cache();
        self.status = format!(
            "{}: {}",
            sev.label(),
            if self.severity_filter[idx] {
                "on"
            } else {
                "off"
            }
        );
        self.selected_idx = 0;
        self.expanded_idx = None;
    }
}
