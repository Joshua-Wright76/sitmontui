use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CanvasLine, Painter, Shape},
        Block, Borders, Clear, Paragraph, Wrap,
    },
    Frame,
};

use crate::{
    app::{App, LayerId, PaneFocus, SearchMode, SearchResult},
    market_ticker::MarketTicker,
    model::{MapObject, ObjectKind, Severity, Warship, WorldLeader},
};

pub fn draw(frame: &mut Frame<'_>, app: &mut App, ticker: &MarketTicker) {
    let root = frame.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(6),
        ])
        .split(root);

    let objects = app.visible_objects();
    render_ticker(frame, chunks[0], ticker);

    // Rotating column layout - selected pane always on left
    match app.focus {
        PaneFocus::Feed => {
            if app.is_map_view {
                // Feed (left 33%) | Map (right 67%)
                let body = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
                    .split(chunks[1]);

                render_feed(frame, body[0], app, objects);
                render_map(frame, body[1], app, objects);
            } else {
                // Feed | Warships | Leaders
                let body = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(33),
                        Constraint::Percentage(34),
                        Constraint::Percentage(33),
                    ])
                    .split(chunks[1]);

                render_feed(frame, body[0], app, objects);
                render_warships(frame, body[1], app);
                render_leaders(frame, body[2], app);
            }
        }
        PaneFocus::Warships => {
            if app.is_map_view {
                // Warships (left 33%) | Map (right 67%)
                let body = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
                    .split(chunks[1]);

                render_warships(frame, body[0], app);
                render_map(frame, body[1], app, objects);
            } else {
                // Warships | Leaders | Feed
                let body = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(33),
                        Constraint::Percentage(34),
                        Constraint::Percentage(33),
                    ])
                    .split(chunks[1]);

                render_warships(frame, body[0], app);
                render_leaders(frame, body[1], app);
                render_feed(frame, body[2], app, objects);
            }
        }
        PaneFocus::Leaders => {
            if app.is_map_view {
                // Leaders (left 33%) | Map (right 67%)
                let body = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(33), Constraint::Percentage(67)])
                    .split(chunks[1]);

                render_leaders(frame, body[0], app);
                render_map(frame, body[1], app, objects);
            } else {
                // Leaders | Feed | Warships
                let body = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(33),
                        Constraint::Percentage(34),
                        Constraint::Percentage(33),
                    ])
                    .split(chunks[1]);

                render_leaders(frame, body[0], app);
                render_feed(frame, body[1], app, objects);
                render_warships(frame, body[2], app);
            }
        }
    }

    render_status(frame, chunks[2], app, objects);

    if app.layer_panel_open {
        render_layer_popup(frame, app);
    }

    if app.is_searching {
        render_search_panel(frame, app);
    }

    if app.filter_panel_open {
        render_filter_popup(frame, chunks[1], app);
    }
}

fn render_ticker(frame: &mut Frame<'_>, area: Rect, ticker: &MarketTicker) {
    let width = area.width as usize;

    // Commodities: gold color
    let commodities_line = ticker.format_line_styled(
        &ticker.commodities,
        ticker.scroll_offset_commodities,
        width,
        Some(Color::Rgb(255, 215, 0)),
    );
    let paragraph1 = Paragraph::new(commodities_line);
    frame.render_widget(
        paragraph1,
        Rect {
            x: area.x,
            y: area.y,
            width: area.width,
            height: 1,
        },
    );

    // Indices/Stocks: cyan color
    let indices_line = ticker.format_line_styled(
        &ticker.indices,
        ticker.scroll_offset_indices,
        width,
        Some(Color::Cyan),
    );
    let paragraph2 = Paragraph::new(indices_line);
    frame.render_widget(
        paragraph2,
        Rect {
            x: area.x,
            y: area.y + 1,
            width: area.width,
            height: 1,
        },
    );

    // Forex/Currencies: light green color
    let forex_line = ticker.format_line_styled(
        &ticker.forex,
        ticker.scroll_offset_forex,
        width,
        Some(Color::Rgb(144, 238, 144)),
    );
    let paragraph3 = Paragraph::new(forex_line);
    frame.render_widget(
        paragraph3,
        Rect {
            x: area.x,
            y: area.y + 2,
            width: area.width,
            height: 1,
        },
    );
}

fn render_map(frame: &mut Frame<'_>, area: Rect, app: &App, objects: &[MapObject]) {
    // Collect events with their categories
    let mut event_markers: Vec<(f64, f64, Option<&str>)> = Vec::new();
    let mut warship_markers: Vec<(f64, f64, &str)> = Vec::new();
    let mut leader_markers: Vec<(f64, f64, &str)> = Vec::new();
    let mut selected_marker: Option<(f64, f64)> = None;

    for (idx, obj) in objects.iter().enumerate() {
        let is_selected = idx == app.selected_idx && app.focus == PaneFocus::Feed;
        if is_selected {
            selected_marker = Some((obj.lng, obj.lat));
        } else {
            event_markers.push((obj.lng, obj.lat, obj.metadata.category.as_deref()));
        }
    }

    for (idx, ship) in app.filtered_warships().iter().enumerate() {
        let is_selected = idx == app.selected_idx_warships && app.focus == PaneFocus::Warships;
        if is_selected {
            selected_marker = Some((ship.lng, ship.lat));
        } else {
            warship_markers.push((ship.lng, ship.lat, &ship.name));
        }
    }

    for (idx, leader) in app.filtered_leaders().iter().enumerate() {
        let is_selected = idx == app.selected_idx_leaders && app.focus == PaneFocus::Leaders;
        if is_selected {
            selected_marker = Some((leader.lng, leader.lat));
        } else {
            leader_markers.push((leader.lng, leader.lat, &leader.name));
        }
    }

    let (center_lng, center_lat) = match app.focus {
        PaneFocus::Feed => app
            .selected_object()
            .filter(|obj| obj.lat != 0.0 && obj.lng != 0.0)
            .map(|obj| (obj.lng, obj.lat))
            .unwrap_or((20.0, 30.0)),
        PaneFocus::Warships => app
            .sorted_warships()
            .get(app.selected_idx_warships)
            .filter(|ship| ship.lat != 0.0 && ship.lng != 0.0)
            .map(|ship| (ship.lng, ship.lat))
            .unwrap_or((20.0, 30.0)),
        PaneFocus::Leaders => app
            .filtered_leaders()
            .get(app.selected_idx_leaders)
            .filter(|leader| leader.lat != 0.0 && leader.lng != 0.0)
            .map(|leader| (leader.lng, leader.lat))
            .unwrap_or((20.0, 30.0)),
    };

    let base_lng_range = 45.0;
    let lng_range = base_lng_range / app.map_zoom_factor;
    let min_lng = (center_lng - lng_range / 2.0).max(-180.0);
    let max_lng = (center_lng + lng_range / 2.0).min(180.0);
    let map_width = area.width as f64;
    let map_height = area.height as f64;
    let aspect_ratio = map_width / (map_height * 2.0);
    let lat_range = lng_range / aspect_ratio;
    let min_lat = (center_lat - lat_range / 2.0).max(-90.0);
    let max_lat = (center_lat + lat_range / 2.0).min(90.0);
    let lod = lod_for_zoom(app.map_zoom_factor);

    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([min_lng, max_lng])
        .y_bounds([min_lat, max_lat])
        .paint(|ctx| {
            let crosshair_color = Color::Rgb(200, 200, 200);
            ctx.draw(&CanvasLine {
                x1: min_lng,
                y1: center_lat,
                x2: max_lng,
                y2: center_lat,
                color: crosshair_color,
            });
            ctx.draw(&CanvasLine {
                x1: center_lng,
                y1: min_lat,
                x2: center_lng,
                y2: max_lat,
                color: crosshair_color,
            });

            draw_coastlines(ctx, min_lng, max_lng, min_lat, max_lat, lod);

            for (lng, lat, category) in &event_markers {
                let emoji = get_event_emoji(*category);
                ctx.print(
                    *lng,
                    *lat,
                    Span::styled(emoji, Style::default().fg(Color::Rgb(255, 255, 0))),
                );
            }

            for (lng, lat, _name) in &warship_markers {
                ctx.print(
                    *lng,
                    *lat,
                    Span::styled("🚢", Style::default().fg(Color::Rgb(0, 255, 255))),
                );
            }

            for (lng, lat, _name) in &leader_markers {
                ctx.print(
                    *lng,
                    *lat,
                    Span::styled("👤", Style::default().fg(Color::Rgb(255, 0, 255))),
                );
            }

            if let Some((lng, lat)) = selected_marker {
                ctx.print(
                    lng,
                    lat,
                    Span::styled("⭐", Style::default().fg(Color::White)),
                );
            }
        })
        .block(Block::default().title("World Map").borders(Borders::ALL));

    frame.render_widget(canvas, area);
    render_mini_map(frame, area, min_lng, max_lng, min_lat, max_lat);
}

/// Get the appropriate emoji for an event based on its category
fn get_event_emoji(category: Option<&str>) -> &'static str {
    match category {
        Some(cat) => {
            let cat_lower = cat.to_lowercase();
            if cat_lower.contains("conflict")
                || cat_lower.contains("military")
                || cat_lower.contains("war")
            {
                "🔥"
            } else if cat_lower.contains("disaster")
                || cat_lower.contains("flood")
                || cat_lower.contains("storm")
            {
                "🌊"
            } else if cat_lower.contains("political")
                || cat_lower.contains("election")
                || cat_lower.contains("protest")
            {
                "🏛️"
            } else if cat_lower.contains("economic")
                || cat_lower.contains("financial")
                || cat_lower.contains("market")
            {
                "💰"
            } else if cat_lower.contains("health")
                || cat_lower.contains("disease")
                || cat_lower.contains("medical")
            {
                "🏥"
            } else if cat_lower.contains("environment")
                || cat_lower.contains("climate")
                || cat_lower.contains("pollution")
            {
                "🌲"
            } else if cat_lower.contains("technology") || cat_lower.contains("innovation") {
                "💻"
            } else if cat_lower.contains("cyber")
                || cat_lower.contains("hack")
                || cat_lower.contains("data")
            {
                "🔒"
            } else if cat_lower.contains("crime")
                || cat_lower.contains("terror")
                || cat_lower.contains("attack")
            {
                "🚨"
            } else {
                "📍"
            }
        }
        None => "📍",
    }
}

/// Draw high-resolution coastlines from Natural Earth data
#[derive(Copy, Clone)]
enum MapLod {
    Low,
    Medium,
    High,
}

struct ThickPolyline<'a> {
    points: &'a [(f64, f64)],
    color: Color,
    radius: i32,
}

impl Shape for ThickPolyline<'_> {
    fn draw(&self, painter: &mut Painter<'_, '_>) {
        if self.points.len() < 2 {
            return;
        }

        for segment in self.points.windows(2) {
            let [start, end] = [segment[0], segment[1]];
            let (x1, y1) = match painter.get_point(start.0, start.1) {
                Some(point) => point,
                None => continue,
            };
            let (x2, y2) = match painter.get_point(end.0, end.1) {
                Some(point) => point,
                None => continue,
            };

            let dx = x2 as i32 - x1 as i32;
            let dy = y2 as i32 - y1 as i32;
            let steps = dx.abs().max(dy.abs()).max(1);

            for step in 0..=steps {
                let t = step as f64 / steps as f64;
                let x = x1 as f64 + (x2 as f64 - x1 as f64) * t;
                let y = y1 as f64 + (y2 as f64 - y1 as f64) * t;
                stamp_brush(
                    painter,
                    x.round() as i32,
                    y.round() as i32,
                    self.radius,
                    self.color,
                );
            }
        }
    }
}

fn stamp_brush(
    painter: &mut Painter<'_, '_>,
    center_x: i32,
    center_y: i32,
    radius: i32,
    color: Color,
) {
    let radius_sq = radius * radius;

    for dx in -radius..=radius {
        for dy in -radius..=radius {
            if dx * dx + dy * dy > radius_sq {
                continue;
            }

            let x = center_x + dx;
            let y = center_y + dy;

            if x < 0 || y < 0 {
                continue;
            }

            painter.paint(x as usize, y as usize, color);
        }
    }
}

fn lod_for_zoom(map_zoom_factor: f64) -> MapLod {
    if map_zoom_factor < 0.75 {
        MapLod::Low
    } else if map_zoom_factor < 2.5 {
        MapLod::Medium
    } else {
        MapLod::High
    }
}

fn draw_coastlines(
    ctx: &mut ratatui::widgets::canvas::Context<'_>,
    min_lng: f64,
    max_lng: f64,
    min_lat: f64,
    max_lat: f64,
    lod: MapLod,
) {
    use crate::coastline_data::{
        BORDER_SEGMENTS_HIGH, BORDER_SEGMENTS_LOW, BORDER_SEGMENTS_MEDIUM, COASTLINE_SEGMENTS_HIGH,
        COASTLINE_SEGMENTS_LOW, COASTLINE_SEGMENTS_MEDIUM, LAKE_SEGMENTS_HIGH, LAKE_SEGMENTS_LOW,
        LAKE_SEGMENTS_MEDIUM,
    };

    let coastline_color = Color::Rgb(100, 200, 255);
    let lake_color = Color::Rgb(80, 180, 240); // Slightly darker blue for lakes
    let border_color = Color::Rgb(120, 120, 120); // Subtle gray for borders
    let (border_segments, coastline_segments, lake_segments) = match lod {
        MapLod::Low => (
            BORDER_SEGMENTS_LOW,
            COASTLINE_SEGMENTS_LOW,
            LAKE_SEGMENTS_LOW,
        ),
        MapLod::Medium => (
            BORDER_SEGMENTS_MEDIUM,
            COASTLINE_SEGMENTS_MEDIUM,
            LAKE_SEGMENTS_MEDIUM,
        ),
        MapLod::High => (
            BORDER_SEGMENTS_HIGH,
            COASTLINE_SEGMENTS_HIGH,
            LAKE_SEGMENTS_HIGH,
        ),
    };

    // Helper to draw segments with a given color
    let draw_segments = |ctx: &mut ratatui::widgets::canvas::Context<'_>,
                         segments: &[&[(f64, f64)]],
                         color: Color,
                         radius: i32| {
        for segment in segments {
            let segment_in_view = segment.iter().any(|(lng, lat)| {
                *lng >= min_lng && *lng <= max_lng && *lat >= min_lat && *lat <= max_lat
            });

            if !segment_in_view {
                continue;
            }

            ctx.draw(&ThickPolyline {
                points: segment,
                color,
                radius,
            });
        }
    };

    let border_radius = match lod {
        MapLod::Low => 0,
        MapLod::Medium => 0,
        MapLod::High => 0,
    };
    let coastline_radius = match lod {
        MapLod::Low => 0,
        MapLod::Medium => 0,
        MapLod::High => 0,
    };
    let lake_radius = match lod {
        MapLod::Low => 0,
        MapLod::Medium => 0,
        MapLod::High => 0,
    };

    // Draw borders first (subtle, behind coastlines)
    draw_segments(ctx, border_segments, border_color, border_radius);

    // Draw coastlines above borders with a slightly larger brush at high zoom.
    draw_segments(ctx, coastline_segments, coastline_color, coastline_radius);

    // Draw lakes last so inland water edges stay readable.
    draw_segments(ctx, lake_segments, lake_color, lake_radius);
}

/// Render mini map in top-right corner showing global context
fn render_mini_map(
    frame: &mut Frame<'_>,
    parent_area: Rect,
    view_min_lng: f64,
    view_max_lng: f64,
    view_min_lat: f64,
    view_max_lat: f64,
) {
    // Mini map size: 22x6 characters (width +50%, height -25%)
    let mini_width = 22u16;
    let mini_height = 6u16;

    let mini_area = Rect {
        x: parent_area.x + parent_area.width - mini_width - 2,
        y: parent_area.y + 1,
        width: mini_width,
        height: mini_height,
    };

    let mini_canvas = Canvas::default()
        .x_bounds([-180.0, 180.0])
        .y_bounds([-90.0, 90.0])
        .marker(Marker::Braille)
        .paint(|ctx| {
            // Draw world map coastlines (dimmed)
            draw_coastlines(ctx, -180.0, 180.0, -90.0, 90.0, MapLod::Low);

            // Draw viewport rectangle showing current view
            let border_color = Color::Rgb(255, 255, 100);

            // Top edge
            ctx.draw(&ratatui::widgets::canvas::Line {
                x1: view_min_lng,
                y1: view_max_lat,
                x2: view_max_lng,
                y2: view_max_lat,
                color: border_color,
            });
            // Bottom edge
            ctx.draw(&ratatui::widgets::canvas::Line {
                x1: view_min_lng,
                y1: view_min_lat,
                x2: view_max_lng,
                y2: view_min_lat,
                color: border_color,
            });
            // Left edge
            ctx.draw(&ratatui::widgets::canvas::Line {
                x1: view_min_lng,
                y1: view_min_lat,
                x2: view_min_lng,
                y2: view_max_lat,
                color: border_color,
            });
            // Right edge
            ctx.draw(&ratatui::widgets::canvas::Line {
                x1: view_max_lng,
                y1: view_min_lat,
                x2: view_max_lng,
                y2: view_max_lat,
                color: border_color,
            });
        })
        .block(Block::default().borders(Borders::NONE));

    frame.render_widget(mini_canvas, mini_area);
}

fn render_feed(frame: &mut Frame<'_>, area: Rect, app: &App, objects: &[MapObject]) {
    let mut lines = Vec::new();
    lines.push(Line::from(vec![
        Span::styled("Feed ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("({} events)", objects.len())),
    ]));

    let selected_idx = app.selected_idx.min(objects.len().saturating_sub(1));
    let is_expanded = app.expanded_idx == Some(selected_idx) && app.focus == PaneFocus::Feed;

    // Calculate visible window to keep selected item in view
    let window_height = area.height.saturating_sub(2) as usize;
    let feed_heights = app.compute_feed_heights(area.width);
    let scroll_offset = calculate_scroll_offset(selected_idx, &feed_heights, window_height);

    for (idx, object) in objects.iter().enumerate().skip(scroll_offset) {
        // Calculate space needed for this item
        let is_selected = idx == selected_idx && app.focus == PaneFocus::Feed;

        // Calculate wrapped title lines
        let title_width = area.width.saturating_sub(7) as usize;
        let wrapped_title = wrap_text(&object.label, title_width);
        let title_lines = wrapped_title.len().min(2); // Max 2 lines for title

        let item_lines = if is_selected && is_expanded {
            // Expanded view takes more space
            8usize
        } else {
            // Normal view: wrapped title lines + details line
            title_lines + 1
        };

        // Check if we have enough space left
        if lines.len() + item_lines > window_height {
            break;
        }

        let prefix = if is_selected { ">" } else { " " };
        let symbol = object_symbol(object, is_selected);
        let style = object_style(object, is_selected);

        // Title lines (wrapped to max 2 lines)
        for (line_idx, title_line) in wrapped_title.iter().take(2).enumerate() {
            if line_idx == 0 {
                // First line has symbol and prefix
                lines.push(Line::from(vec![
                    Span::raw(prefix),
                    Span::raw(" "),
                    Span::styled(symbol, style),
                    Span::raw(" "),
                    Span::styled(title_line.clone(), style),
                ]));
            } else {
                // Subsequent lines are indented
                lines.push(Line::from(vec![
                    Span::raw("     "), // 5 spaces to align with title text
                    Span::styled(title_line.clone(), style),
                ]));
            }
        }

        // Details line (time, category, signals)
        if is_selected && is_expanded {
            render_expanded_details(&mut lines, object, app, area.width);
        } else {
            render_compact_details(&mut lines, object);
        }

        // Add spacing between items
        if idx < objects.len() - 1 && lines.len() < window_height {
            lines.push(Line::from(""));
        }
    }

    if objects.is_empty() {
        lines.push(Line::from(Span::styled(
            "No events available",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let feed_title = if app.focus == PaneFocus::Feed {
        "Feed [ACTIVE]"
    } else {
        "Feed"
    };

    let panel = Paragraph::new(lines)
        .block(Block::default().title(feed_title).borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(panel, area);
}

fn render_compact_details(lines: &mut Vec<Line<'_>>, object: &MapObject) {
    let meta = &object.metadata;
    let mut parts = Vec::new();

    // Time - use pre-parsed timestamp for efficiency
    if let Some(timestamp) = object.timestamp {
        let time_str = format_timestamp_relative(timestamp);
        parts.push(Span::styled(
            format!("📅 {}", time_str),
            Style::default().fg(Color::Gray),
        ));
    }

    // Category/Type
    if let Some(ref cat) = meta.category {
        parts.push(Span::raw(" | "));
        parts.push(Span::styled(
            format!("🏷️ {}", cat),
            Style::default().fg(Color::Gray),
        ));
    } else if object.kind == ObjectKind::Aircraft {
        parts.push(Span::raw(" | "));
        parts.push(Span::styled(
            "✈️ aircraft",
            Style::default().fg(Color::Gray),
        ));
    }

    // Signal count for events
    if let Some(count) = meta.signal_count {
        if count > 0 {
            parts.push(Span::raw(" | "));
            parts.push(Span::styled(
                format!("📡 {} signals", count),
                Style::default().fg(Color::Gray),
            ));
        }
    }

    // Aircraft details
    if object.kind == ObjectKind::Aircraft {
        if let Some(alt) = meta.altitude {
            parts.push(Span::raw(" | "));
            parts.push(Span::styled(
                format!("🛫 {} ft", alt),
                Style::default().fg(Color::Gray),
            ));
        }
        if let Some(speed) = meta.speed {
            parts.push(Span::raw(" | "));
            parts.push(Span::styled(
                format!("⚡ {} kts", speed),
                Style::default().fg(Color::Gray),
            ));
        }
    }

    if !parts.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("    "),
            Span::styled("└─ ", Style::default().fg(Color::DarkGray)),
        ]));
        // Add parts to the last line
        if let Some(last_line) = lines.last_mut() {
            last_line.spans.extend(parts);
        }
    }
}

fn render_expanded_details(lines: &mut Vec<Line<'_>>, object: &MapObject, app: &App, width: u16) {
    let meta = &object.metadata;
    let indent = "     ";

    // Separator line
    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(
            "─".repeat(width.saturating_sub(6) as usize),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Summary/Description
    if let Some(ref summary) = meta.summary {
        if !summary.is_empty() {
            let wrapped = wrap_text(summary, width.saturating_sub(6) as usize);
            for line in wrapped {
                lines.push(Line::from(vec![
                    Span::raw(indent),
                    Span::styled(line, Style::default().fg(Color::White)),
                ]));
            }
            lines.push(Line::from(""));
        }
    }

    // Location details
    let mut location_parts = Vec::new();
    if let Some(ref loc) = meta.location {
        location_parts.push(format!("📍 {}", loc));
    }
    if let Some(ref country) = meta.country {
        location_parts.push(format!("🌍 {}", country));
    }
    if let Some(ref region) = meta.region {
        if meta.country.is_none() {
            location_parts.push(format!("🌍 {}", region));
        }
    }
    if !location_parts.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(indent),
            Span::styled(
                location_parts.join(" | "),
                Style::default().fg(Color::Yellow),
            ),
        ]));
    }

    // Coordinates
    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(
            format!("🌐 {:.4}°, {:.4}°", object.lat, object.lng),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    // Metadata details
    let mut meta_parts = Vec::new();

    if let Some(ref created) = meta.created_at {
        meta_parts.push(format!("🕐 {}", created));
    }
    if let Some(count) = meta.signal_count {
        meta_parts.push(format!("📊 {} signals", count));
    }
    if let Some(conf) = meta.confidence {
        meta_parts.push(format!("✓ {}% confidence", conf));
    }
    if let Some(ref sources) = meta.source_types {
        meta_parts.push(format!("📡 {}", sources));
    }

    if !meta_parts.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(indent),
            Span::styled(meta_parts.join(" | "), Style::default().fg(Color::Gray)),
        ]));
    }

    // Signals breakdown
    if let Some(signals) = app.get_signals_for_event(&object.id) {
        if !signals.is_empty() {
            // Group signals by source type
            let mut source_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for signal in &signals {
                *source_counts.entry(signal.source_type.clone()).or_insert(0) += 1;
            }

            lines.push(Line::from(vec![
                Span::raw(indent),
                Span::styled("📡 Signals:", Style::default().fg(Color::White)),
            ]));

            for (source, count) in source_counts.iter() {
                let source_label = match source.as_str() {
                    "social_media" => "social media",
                    "news" => "news",
                    "government" => "government",
                    "local" => "local",
                    "corporate" => "corporate",
                    _ => source.as_str(),
                };
                lines.push(Line::from(vec![
                    Span::raw(indent),
                    Span::styled("  └─ ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{}: {}", source_label, count),
                        Style::default().fg(Color::Gray),
                    ),
                ]));
            }

            // Show individual signal content (up to 4 signals, each can wrap to multiple lines)
            let content_width = (width.saturating_sub(8)) as usize;
            for signal in signals.iter().take(4) {
                let wrapped_content = wrap_text(&signal.content, content_width);
                for (i, line) in wrapped_content.iter().take(4).enumerate() {
                    let prefix = if i == 0 { "  └─ " } else { "     " };
                    lines.push(Line::from(vec![
                        Span::raw(indent),
                        Span::styled(prefix, Style::default().fg(Color::DarkGray)),
                        Span::styled(line.clone(), Style::default().fg(Color::LightBlue)),
                    ]));
                }
            }
        }
    } else if meta.signal_count.unwrap_or(0) > 0
        && object.kind == crate::model::ObjectKind::Incident
    {
        // Signals not yet loaded, show loading indicator
        lines.push(Line::from(vec![
            Span::raw(indent),
            Span::styled("📡 Signals: ", Style::default().fg(Color::White)),
            Span::styled("Loading...", Style::default().fg(Color::Gray)),
        ]));
    }

    // Aircraft-specific details
    if object.kind == ObjectKind::Aircraft {
        let mut ac_parts = Vec::new();
        if let Some(alt) = meta.altitude {
            ac_parts.push(format!("🛫 Altitude: {} ft", alt));
        }
        if let Some(speed) = meta.speed {
            ac_parts.push(format!("⚡ Speed: {} kts", speed));
        }
        if let Some(hdg) = meta.heading {
            ac_parts.push(format!("🧭 Heading: {}°", hdg));
        }
        if let Some(ref ac_type) = meta.aircraft_type {
            ac_parts.push(format!("✈️ Type: {}", ac_type));
        }
        if let Some(ref callsign) = meta.callsign {
            ac_parts.push(format!("📻 Callsign: {}", callsign));
        }

        if !ac_parts.is_empty() {
            lines.push(Line::from(vec![
                Span::raw(indent),
                Span::styled(ac_parts.join(" | "), Style::default().fg(Color::LightBlue)),
            ]));
        }
    }

    // Category/Subtype
    if let Some(ref cat) = meta.category {
        let subtype_str = meta
            .subtype
            .as_ref()
            .map(|s| format!("/ {}", s))
            .unwrap_or_default();
        lines.push(Line::from(vec![
            Span::raw(indent),
            Span::styled(
                format!("🏷️ Category: {} {}", cat, subtype_str),
                Style::default().fg(Color::Magenta),
            ),
        ]));
    }
}

fn render_warships(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut lines = Vec::new();
    let mut warships: Vec<_> = app.snapshot.warships.clone();

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

    let selected_idx = app
        .selected_idx_warships
        .min(warships.len().saturating_sub(1));
    let is_expanded = app.expanded_idx == Some(selected_idx) && app.focus == PaneFocus::Warships;
    let window_height = area.height.saturating_sub(2) as usize;

    // Compute item heights: 3 lines normal, 8 lines if expanded
    let warships_heights: Vec<usize> = warships
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if is_expanded && i == selected_idx {
                8
            } else {
                3
            }
        })
        .collect();
    let scroll_offset = calculate_scroll_offset(selected_idx, &warships_heights, window_height);

    // Calculate visible range for position indicator
    let visible_start = scroll_offset + 1;
    let visible_end = (scroll_offset + window_height / 3).min(warships.len());
    let position_str = if warships.len() > window_height / 3 {
        format!(" [{}-{}/{}]", visible_start, visible_end, warships.len())
    } else {
        String::new()
    };

    lines.push(Line::from(vec![
        Span::styled("Warships ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!("({} ships)", warships.len())),
        Span::styled(position_str, Style::default().fg(Color::DarkGray)),
    ]));

    for (idx, ship) in warships.iter().enumerate().skip(scroll_offset) {
        let is_selected = idx == selected_idx && app.focus == PaneFocus::Warships;

        // Check if we have enough space
        let item_lines = if is_selected && is_expanded { 8 } else { 3 };
        if lines.len() + item_lines > window_height {
            break;
        }

        let prefix = if is_selected { ">" } else { " " };

        let flag = country_flag(&ship.country);
        let name_style = if ship.ship_type.to_lowercase().contains("carrier") {
            if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Rgb(20, 20, 20))
            } else {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            }
        } else if is_selected {
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(20, 20, 20))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let type_str = if let Some(ref hull) = ship.hull_number {
            format!("{} ({})", ship.ship_type, hull)
        } else {
            ship.ship_type.clone()
        };

        lines.push(Line::from(vec![
            Span::raw(prefix),
            Span::raw(" "),
            Span::raw(format!("{} ", flag)),
            Span::styled(&ship.name, name_style),
            Span::raw(" · "),
            Span::styled(type_str, Style::default().fg(Color::Gray)),
        ]));

        let status_emoji = match ship.status.as_str() {
            "deployed" => "⚓",
            "transiting" => "🚢",
            _ => "📍",
        };
        lines.push(Line::from(Span::styled(
            format!("    {} {} | {}", status_emoji, ship.region, ship.status),
            Style::default().fg(Color::Cyan),
        )));

        if is_selected && is_expanded {
            render_warship_details(&mut lines, ship, area.width);
        }

        if idx < warships.len() - 1 {
            lines.push(Line::from(""));
        }
    }

    if warships.is_empty() {
        lines.push(Line::from(Span::styled(
            "No warship data available",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let panel_title = if app.focus == PaneFocus::Warships {
        "Warships [ACTIVE]"
    } else {
        "Warships"
    };

    let panel = Paragraph::new(lines)
        .block(Block::default().title(panel_title).borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(panel, area);
}

fn render_warship_details(lines: &mut Vec<Line<'_>>, ship: &Warship, width: u16) {
    let indent = "     ";

    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(
            "─".repeat(width.saturating_sub(6) as usize),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(
            format!("🌐 {:.4}°, {:.4}°", ship.lat, ship.lng),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    let mut detail_parts = Vec::new();

    if let Some(ref group_name) = ship.group_name {
        detail_parts.push(format!("🚢 Group: {}", group_name));
    }
    if let Some(ref group_type) = ship.group_type {
        detail_parts.push(format!("📋 Type: {}", group_type));
    }
    if ship.flagship {
        detail_parts.push("🏴 Flagship".to_string());
    }

    if !detail_parts.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(indent),
            Span::styled(
                detail_parts.join(" | "),
                Style::default().fg(Color::LightBlue),
            ),
        ]));
    }

    let mut source_parts = Vec::new();
    if let Some(ref url) = ship.source_url {
        if !url.is_empty() {
            source_parts.push("🔗 Source available".to_string());
        }
    }
    if let Some(ref date) = ship.source_date {
        if !date.is_empty() {
            source_parts.push(format!("📅 {}", date));
        }
    }
    if !source_parts.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(indent),
            Span::styled(source_parts.join(" | "), Style::default().fg(Color::Gray)),
        ]));
    }

    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(
            format!("🕐 Updated: {}", ship.updated_at),
            Style::default().fg(Color::Gray),
        ),
    ]));
}

fn render_leaders(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut lines = Vec::new();
    let leaders: Vec<_> = app.snapshot.leaders.clone();

    let selected_idx = app
        .selected_idx_leaders
        .min(leaders.len().saturating_sub(1));
    let is_expanded = app.expanded_idx == Some(selected_idx) && app.focus == PaneFocus::Leaders;
    let window_height = area.height.saturating_sub(2) as usize;

    // Compute item heights: 4 lines normal, 8 lines if expanded
    let leaders_heights: Vec<usize> = leaders
        .iter()
        .enumerate()
        .map(|(i, _)| {
            if is_expanded && i == selected_idx {
                8
            } else {
                4
            }
        })
        .collect();
    let scroll_offset = calculate_scroll_offset(selected_idx, &leaders_heights, window_height);

    // Calculate visible range for position indicator
    let visible_start = scroll_offset + 1;
    let visible_end = (scroll_offset + window_height / 4).min(leaders.len());
    let position_str = if leaders.len() > window_height / 4 {
        format!(" [{}-{}/{}]", visible_start, visible_end, leaders.len())
    } else {
        String::new()
    };

    lines.push(Line::from(vec![
        Span::styled(
            "World Leaders ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!("({} leaders)", leaders.len())),
        Span::styled(position_str, Style::default().fg(Color::DarkGray)),
    ]));

    for (idx, leader) in leaders.iter().enumerate().skip(scroll_offset) {
        let is_selected = idx == selected_idx && app.focus == PaneFocus::Leaders;

        // Check if we have enough space
        let item_lines = if is_selected && is_expanded { 8 } else { 4 };
        if lines.len() + item_lines > window_height {
            break;
        }

        let prefix = if is_selected { ">" } else { " " };

        let flag = country_flag(&leader.country_code);
        let name_style = if is_selected {
            Style::default()
                .fg(Color::White)
                .bg(Color::Rgb(20, 20, 20))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        };

        lines.push(Line::from(vec![
            Span::raw(prefix),
            Span::raw(" "),
            Span::raw(format!("{} ", flag)),
            Span::styled(&leader.name, name_style),
            Span::raw(" · "),
            Span::styled(&leader.title, Style::default().fg(Color::Gray)),
        ]));

        let activity_short = if leader.activity.len() > 50 {
            format!("{}...", &leader.activity[..47])
        } else {
            leader.activity.clone()
        };
        lines.push(Line::from(Span::styled(
            format!("    📍 {} · {}", leader.location_name, activity_short),
            Style::default().fg(Color::Cyan),
        )));

        let confidence_emoji = match leader.confidence.as_str() {
            "high" => "✓",
            "medium" => "~",
            _ => "?",
        };
        let confidence_color = match leader.confidence.as_str() {
            "high" => Color::Green,
            "medium" => Color::Yellow,
            _ => Color::Red,
        };
        lines.push(Line::from(Span::styled(
            format!("    {} Confidence: {}", confidence_emoji, leader.confidence),
            Style::default().fg(confidence_color),
        )));

        if is_selected && is_expanded {
            render_leader_details(&mut lines, leader, area.width);
        }

        if idx < leaders.len() - 1 {
            lines.push(Line::from(""));
        }
    }

    if leaders.is_empty() {
        lines.push(Line::from(Span::styled(
            "No leader data available",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let panel_title = if app.focus == PaneFocus::Leaders {
        "World Leaders [ACTIVE]"
    } else {
        "World Leaders"
    };

    let panel = Paragraph::new(lines)
        .block(Block::default().title(panel_title).borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(panel, area);
}

fn render_leader_details(lines: &mut Vec<Line<'_>>, leader: &WorldLeader, width: u16) {
    let indent = "     ";

    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(
            "─".repeat(width.saturating_sub(6) as usize),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(
            format!("🌐 {:.4}°, {:.4}°", leader.lat, leader.lng),
            Style::default().fg(Color::Cyan),
        ),
    ]));

    if let Some(ref next_activity) = leader.next_activity {
        if !next_activity.is_empty() {
            lines.push(Line::from(vec![
                Span::raw(indent),
                Span::styled(
                    format!("📅 Next: {}", next_activity),
                    Style::default().fg(Color::Yellow),
                ),
            ]));
        }
    }

    if !leader.source_summary.is_empty() {
        let wrapped = wrap_text(&leader.source_summary, width.saturating_sub(6) as usize);
        for line in wrapped.iter().take(3) {
            lines.push(Line::from(vec![
                Span::raw(indent),
                Span::styled(line.clone(), Style::default().fg(Color::Gray)),
            ]));
        }
    }

    lines.push(Line::from(vec![
        Span::raw(indent),
        Span::styled(
            format!("🕐 Updated: {}", leader.updated_at),
            Style::default().fg(Color::Gray),
        ),
    ]));
}

fn render_status(frame: &mut Frame<'_>, area: Rect, app: &App, objects: &[MapObject]) {
    let selected_text = app
        .selected_object()
        .map(|o| format!("selected: {}", o.label))
        .unwrap_or_else(|| String::from("selected: none"));

    let filters = format!(
        "inc:{} ship:{} sev[1:{} 2:{} 3:{} 4:{}]",
        on_off(app.layer_visible(LayerId::Incidents)),
        on_off(app.layer_visible(LayerId::Ships)),
        on_off(app.severity_filter[0]),
        on_off(app.severity_filter[1]),
        on_off(app.severity_filter[2]),
        on_off(app.severity_filter[3])
    );

    let pane_keys = match app.focus {
        PaneFocus::Feed => "ACTIVE PANE: FEED  |  j/k move selection  |  h/l move focus",
        PaneFocus::Warships => "ACTIVE PANE: WARSHIPS  |  j/k move selection  |  h/l move focus",
        PaneFocus::Leaders => "ACTIVE PANE: LEADERS  |  j/k move selection  |  h/l move focus",
    };

    let nav_keys = if app.filter_panel_open {
        "f: close | j/k: move | ENTER: toggle"
    } else if app.is_map_view {
        "/: search | m: list view | ENTER: details | gg/G: top/bottom | 1-4: severity | q: quit"
    } else {
        "/: search | f: filters | m: map view | ENTER: details | gg/G: top/bottom | 1-4: severity | q: quit"
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&app.status),
            Span::raw("  |  "),
            Span::raw(selected_text),
            Span::raw("  |  "),
            Span::raw(format!("events: {}", objects.len())),
            Span::raw("  |  "),
            Span::raw(format!("warships: {}", app.snapshot.warships.len())),
            Span::raw("  |  "),
            Span::raw(format!("leaders: {}", app.snapshot.leaders.len())),
        ]),
        Line::from(Span::styled(pane_keys, Style::default().fg(Color::Gray))),
        Line::from(Span::styled(nav_keys, Style::default().fg(Color::DarkGray))),
        Line::from(filters),
    ];

    let panel = Paragraph::new(lines).block(Block::default().title("Ops").borders(Borders::ALL));
    frame.render_widget(panel, area);
}

fn render_layer_popup(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(8).min(54).max(36);
    let popup_h = area.height.saturating_sub(6).min(14).max(10);
    let popup = Rect {
        x: area.x + area.width.saturating_sub(popup_w) / 2,
        y: area.y + area.height.saturating_sub(popup_h) / 2,
        width: popup_w,
        height: popup_h,
    };

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "Layer Controls",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "j/k move  space toggle  a all  d defaults  esc close",
        Style::default().fg(Color::Gray),
    )));
    lines.push(Line::from(""));

    for (idx, layer) in app.layers.iter().enumerate() {
        let cursor = if idx == app.layer_cursor { ">" } else { " " };
        let mark = if layer.visible { "[x]" } else { "[ ]" };
        lines.push(Line::from(format!("{} {} {}", cursor, mark, layer.name)));
    }

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title("Layer Panel")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Rgb(10, 14, 24))),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, popup);
    frame.render_widget(widget, popup);
}

fn column_rect(area: Rect, index: usize) -> Rect {
    let col_width = area.width / 3;
    Rect {
        x: area.x + (index as u16) * col_width,
        y: area.y,
        width: col_width,
        height: area.height,
    }
}

fn render_filter_popup(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let col_index = match app.focus {
        PaneFocus::Feed => 0,
        PaneFocus::Warships => 1,
        PaneFocus::Leaders => 2,
    };
    let col = column_rect(area, col_index);

    let filter_count = app.filter_count();
    if filter_count == 0 {
        return;
    }

    let popup_w = 28.min(col.width.saturating_sub(2));
    let popup_h = ((filter_count + 4) as u16).min(col.height.saturating_sub(2));

    let popup = Rect {
        x: col.x + (col.width - popup_w) / 2,
        y: col.y + (col.height - popup_h) / 2,
        width: popup_w,
        height: popup_h,
    };

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "Filters",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    let filters: Vec<(&str, bool)> = match app.focus {
        PaneFocus::Feed => vec![
            ("Live", app.feed_filters.show_live),
            ("Reports", app.feed_filters.show_reports),
        ],
        _ => vec![],
    };

    for (idx, (label, enabled)) in filters.iter().enumerate() {
        let cursor = if idx == app.filter_selection_idx {
            ">"
        } else {
            " "
        };
        let mark = if *enabled { "[●]" } else { "[○]" };
        lines.push(Line::from(format!("{} {} {}", cursor, mark, label)));
    }

    let widget = Paragraph::new(lines)
        .block(
            Block::default()
                .title(format!("{:?} FILTERS", app.focus).to_uppercase())
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, popup);
    frame.render_widget(widget, popup);
}

fn render_search_panel(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    let popup_w = area.width.saturating_sub(10).min(80).max(60);
    let popup_h = area.height.saturating_sub(8).min(20).max(10);
    let popup = Rect {
        x: area.x + area.width.saturating_sub(popup_w) / 2,
        y: area.y + area.height.saturating_sub(popup_h) / 2,
        width: popup_w,
        height: popup_h,
    };

    let mut lines = Vec::new();

    match app.search_mode {
        SearchMode::Input => {
            lines.push(Line::from(Span::styled(
                "Search",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "Type query to search  |  ENTER to view results  |  ESC to close",
                Style::default().fg(Color::Gray),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::raw("Query: "),
                Span::raw(&app.search_query),
                Span::raw("_"),
            ]));
            lines.push(Line::from(""));

            if !app.search_query.is_empty() {
                lines.push(Line::from(Span::styled(
                    "Results:",
                    Style::default().add_modifier(Modifier::BOLD),
                )));
                for (idx, result) in app.search_results.iter().enumerate() {
                    let prefix = if idx == app.search_selected_idx {
                        ">"
                    } else {
                        " "
                    };
                    let line = match result {
                        SearchResult::Feed(i) => {
                            let obj = app.visible_objects().get(*i);
                            format!(
                                "{} Feed: {}",
                                prefix,
                                obj.map(|o| o.label.as_str()).unwrap_or("?")
                            )
                        }
                        SearchResult::Warship(i) => {
                            let ship = app.snapshot.warships.get(*i);
                            format!(
                                "{} Warship: {}",
                                prefix,
                                ship.map(|s| s.name.as_str()).unwrap_or("?")
                            )
                        }
                        SearchResult::Leader(i) => {
                            let leader = app.snapshot.leaders.get(*i);
                            format!(
                                "{} Leader: {}",
                                prefix,
                                leader.map(|l| l.name.as_str()).unwrap_or("?")
                            )
                        }
                    };
                    lines.push(Line::from(line));
                }
            }
        }
        SearchMode::Results => {
            lines.push(Line::from(Span::styled(
                "Search Results",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(Span::styled(
                "j/k to navigate  |  ENTER to select  |  ESC to go back",
                Style::default().fg(Color::Gray),
            )));
            lines.push(Line::from(vec![
                Span::raw("Query: "),
                Span::raw(&app.search_query),
            ]));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Results:",
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for (idx, result) in app.search_results.iter().enumerate() {
                let prefix = if idx == app.search_selected_idx {
                    ">"
                } else {
                    " "
                };
                let (source, content) = match result {
                    SearchResult::Feed(i) => {
                        let obj = app.visible_objects().get(*i);
                        let label = obj.map(|o| o.label.as_str()).unwrap_or("?");
                        let cat = obj
                            .and_then(|o| o.metadata.category.as_ref())
                            .map(|s| s.as_str())
                            .unwrap_or("");
                        ("Feed", format!("{} | {}", label, cat))
                    }
                    SearchResult::Warship(i) => {
                        let ship = app.snapshot.warships.get(*i);
                        let name = ship.map(|s| s.name.as_str()).unwrap_or("?");
                        let ship_type = ship.map(|s| s.ship_type.as_str()).unwrap_or("");
                        ("Warship", format!("{} | {}", name, ship_type))
                    }
                    SearchResult::Leader(i) => {
                        let leader = app.snapshot.leaders.get(*i);
                        let name = leader.map(|l| l.name.as_str()).unwrap_or("?");
                        let title = leader.map(|l| l.title.as_str()).unwrap_or("");
                        ("Leader", format!("{} | {}", name, title))
                    }
                };
                lines.push(Line::from(format!("{} [{}] {}", prefix, source, content)));
            }
        }
    }

    let widget = Paragraph::new(lines)
        .block(Block::default().title("Search Panel").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(Clear, popup);
    frame.render_widget(widget, popup);
}

/// Calculate the scroll offset to keep the selected item visible in the feed window.
/// Uses cumulative height estimation to center the selected item in the window.
fn calculate_scroll_offset(selected_idx: usize, heights: &[usize], window_height: usize) -> usize {
    if heights.is_empty() || selected_idx >= heights.len() {
        return 0;
    }

    // If first item is selected, always start from beginning
    if selected_idx == 0 {
        return 0;
    }

    // Calculate cumulative height up to the selected item
    let mut cumulative_height = 0usize;
    for i in 0..selected_idx {
        cumulative_height += heights[i];
    }

    // Target scroll position to center the selected item in the window
    let target_y = cumulative_height.saturating_sub(window_height / 2);

    // Find scroll_offset where cumulative height exceeds target_y
    let mut scroll_offset = 0;
    cumulative_height = 0;

    for (idx, &height) in heights.iter().enumerate() {
        if cumulative_height >= target_y {
            break;
        }
        scroll_offset = idx + 1;
        cumulative_height += height;
    }

    scroll_offset.min(heights.len().saturating_sub(1))
}

fn format_timestamp_relative(timestamp: i64) -> String {
    use std::time::SystemTime;

    let now_secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let diff_secs = now_secs - timestamp;

    if diff_secs < 0 {
        return "in future".to_string();
    }

    format_relative_time(diff_secs as u64)
}

fn format_relative_time(seconds: u64) -> String {
    const MINUTE: u64 = 60;
    const HOUR: u64 = 60 * MINUTE;
    const DAY: u64 = 24 * HOUR;
    const WEEK: u64 = 7 * DAY;

    match seconds {
        s if s < MINUTE => format!("{}s ago", s),
        s if s < 2 * MINUTE => "1 min ago".to_string(),
        s if s < HOUR => format!("{}m ago", s / MINUTE),
        s if s < 2 * HOUR => "1h ago".to_string(),
        s if s < DAY => format!("{}h ago", s / HOUR),
        s if s < 2 * DAY => "1d ago".to_string(),
        s if s < WEEK => format!("{}d ago", s / DAY),
        s => format!("{}w ago", s / WEEK),
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut current_line = String::new();

    for word in text.split_whitespace() {
        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_line.len() + 1 + word.len() <= width {
            current_line.push(' ');
            current_line.push_str(word);
        } else {
            result.push(current_line);
            current_line = word.to_string();
        }
    }

    if !current_line.is_empty() {
        result.push(current_line);
    }

    result
}

fn object_symbol(object: &MapObject, selected: bool) -> &'static str {
    if selected {
        return "⬢";
    }
    match object.kind {
        ObjectKind::Incident => match object.severity {
            Some(Severity::Low) => "·",
            Some(Severity::Medium) => "!",
            Some(Severity::High) => "▲",
            Some(Severity::Critical) => "◆",
            None => "·",
        },
        ObjectKind::Ship => "⛴",
        ObjectKind::Aircraft => "✈️",
    }
}

fn object_style(object: &MapObject, selected: bool) -> Style {
    // Color-code based on confidence percentage
    let confidence = object.metadata.confidence;

    let color = match confidence {
        None => Color::White,
        Some(c) if c >= 90 => Color::Rgb(86, 180, 233), // Blue
        Some(c) if c >= 70 => Color::Rgb(0, 158, 115),  // Green
        Some(c) if c >= 50 => Color::Rgb(240, 228, 66), // Yellow
        Some(c) if c >= 30 => Color::Rgb(230, 159, 0),  // Orange
        _ => Color::Rgb(213, 94, 0),                    // Red
    };

    if selected {
        return Style::default()
            .fg(color)
            .bg(Color::Rgb(20, 20, 20))
            .add_modifier(Modifier::BOLD);
    }

    Style::default().fg(color)
}

fn country_flag(country_code: &str) -> String {
    // Return bracketed country code instead of emoji
    // Emojis don't render properly in many terminals, showing as letters instead
    let code = country_code.to_uppercase();
    if code.len() == 2 {
        format!("[{}]", code)
    } else {
        "[??]".to_string()
    }
}

fn on_off(v: bool) -> &'static str {
    if v {
        "on"
    } else {
        "off"
    }
}
