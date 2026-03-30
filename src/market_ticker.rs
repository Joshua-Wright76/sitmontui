use ratatui::{style::Style, text::Line};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketItem {
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub change_1h: f64,
    pub change_24h: f64,
    pub change_1w: f64,
    pub change_1m: f64,
    pub show_hourly: bool,
}

impl MarketItem {
    pub fn new(
        symbol: &str,
        name: &str,
        price: f64,
        change_1h: f64,
        change_24h: f64,
        change_1w: f64,
        change_1m: f64,
        show_hourly: bool,
    ) -> Self {
        Self {
            symbol: symbol.to_string(),
            name: name.to_string(),
            price,
            change_1h,
            change_24h,
            change_1w,
            change_1m,
            show_hourly,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketTicker {
    pub commodities: Vec<MarketItem>,
    pub indices: Vec<MarketItem>,
    pub forex: Vec<MarketItem>,
    pub scroll_offset_commodities: usize,
    pub scroll_offset_indices: usize,
    pub scroll_offset_forex: usize,
    pub last_scroll_ms_commodities: u64,
    pub last_scroll_ms_indices: u64,
    pub last_scroll_ms_forex: u64,
    pub last_fetch_ms: u64,
}

impl MarketTicker {
    pub fn new() -> Self {
        Self {
            commodities: Vec::new(),
            indices: Vec::new(),
            forex: Vec::new(),
            scroll_offset_commodities: 0,
            scroll_offset_indices: 0,
            scroll_offset_forex: 0,
            last_scroll_ms_commodities: 0,
            last_scroll_ms_indices: 0,
            last_scroll_ms_forex: 0,
            last_fetch_ms: 0,
        }
    }

    pub fn maybe_scroll(&mut self, now_ms: u64) {
        if now_ms - self.last_scroll_ms_commodities >= 500 {
            self.scroll_offset_commodities = self.scroll_offset_commodities.wrapping_add(1);
            self.last_scroll_ms_commodities = now_ms;
        }
        if now_ms - self.last_scroll_ms_indices >= 500 {
            self.scroll_offset_indices = self.scroll_offset_indices.wrapping_sub(1);
            self.last_scroll_ms_indices = now_ms;
        }
        if now_ms - self.last_scroll_ms_forex >= 500 {
            self.scroll_offset_forex = self.scroll_offset_forex.wrapping_add(1);
            self.last_scroll_ms_forex = now_ms;
        }
    }

    pub fn should_refetch(&self, now_ms: u64) -> bool {
        now_ms - self.last_fetch_ms >= 5 * 60 * 1000
    }

    pub fn fetch_quotes(&mut self, now_ms: u64) -> anyhow::Result<()> {
        if !self.should_refetch(now_ms) {
            return Ok(());
        }

        let commodity_symbols = vec![
            ("GC=F", "Gold"),
            ("SI=F", "Silver"),
            ("CL=F", "WTI Oil"),
            ("BZ=F", "Brent"),
            ("NG=F", "Nat Gas"),
            ("BTC-USD", "Bitcoin"),
            ("ETH-USD", "Ethereum"),
        ];

        let index_symbols = vec![
            ("^GSPC", "S&P 500"),
            ("^DJI", "Dow Jones"),
            ("^TNX", "10Y Treasury"),
            ("^TYX", "30Y Treasury"),
        ];

        let forex_symbols = vec![
            ("EURUSD=X", "USD/EUR"),
            ("USDJPY=X", "USD/JPY"),
            ("USDCNY=X", "USD/CNY"),
            ("USDRUB=X", "USD/RUB"),
            ("USDINR=X", "USD/INR"),
        ];

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        let mut commodities = Vec::new();
        for (symbol, name) in &commodity_symbols {
            match fetch_all_changes(&client, symbol, true) {
                Ok(item) => commodities.push(item),
                Err(e) => {
                    eprintln!("Failed to fetch {}: {}", symbol, e);
                    commodities.push(MarketItem::new(symbol, name, 0.0, 0.0, 0.0, 0.0, 0.0, true));
                }
            }
        }

        let mut indices = Vec::new();
        for (symbol, name) in &index_symbols {
            match fetch_all_changes(&client, symbol, true) {
                Ok(item) => indices.push(item),
                Err(e) => {
                    eprintln!("Failed to fetch {}: {}", symbol, e);
                    indices.push(MarketItem::new(symbol, name, 0.0, 0.0, 0.0, 0.0, 0.0, true));
                }
            }
        }

        let mut forex = Vec::new();
        for (symbol, name) in &forex_symbols {
            match fetch_all_changes(&client, symbol, false) {
                Ok(item) => forex.push(item),
                Err(e) => {
                    eprintln!("Failed to fetch {}: {}", symbol, e);
                    forex.push(MarketItem::new(
                        symbol, name, 0.0, 0.0, 0.0, 0.0, 0.0, false,
                    ));
                }
            }
        }

        self.commodities = commodities;
        self.indices = indices;
        self.forex = forex;
        self.last_fetch_ms = now_ms;
        Ok(())
    }

    pub fn format_line_styled(
        &self,
        items: &[MarketItem],
        scroll_offset: usize,
        width: usize,
        name_color: Option<ratatui::style::Color>,
    ) -> ratatui::text::Line<'static> {
        use ratatui::style::Color;
        use ratatui::text::Span;

        if items.is_empty() {
            return Line::default();
        }

        #[derive(Clone)]
        struct StyledSpan {
            text: String,
            color: Option<Color>,
        }

        let mut styled_spans: Vec<StyledSpan> = Vec::new();

        for (item_idx, item) in items.iter().enumerate() {
            if item.price == 0.0 {
                continue;
            }

            let price_str = format_price(item.price);

            styled_spans.push(StyledSpan {
                text: format!("{} ", item.name),
                color: name_color,
            });
            styled_spans.push(StyledSpan {
                text: format!("{} ", price_str),
                color: None,
            });

            let pct_color = |val: f64| -> Color {
                if val >= 0.0 {
                    Color::Green
                } else {
                    Color::Red
                }
            };

            if item.show_hourly {
                styled_spans.push(StyledSpan {
                    text: format!("🕐{:+.2}% ", item.change_1h),
                    color: Some(pct_color(item.change_1h)),
                });
            }
            styled_spans.push(StyledSpan {
                text: format!("☀️{:+.2}% ", item.change_24h),
                color: Some(pct_color(item.change_24h)),
            });
            styled_spans.push(StyledSpan {
                text: format!("W{:+.2}% ", item.change_1w),
                color: Some(pct_color(item.change_1w)),
            });
            styled_spans.push(StyledSpan {
                text: format!("🌙{:+.2}% ", item.change_1m),
                color: Some(pct_color(item.change_1m)),
            });

            if item_idx < items.len() - 1 {
                styled_spans.push(StyledSpan {
                    text: " | ".to_string(),
                    color: None,
                });
            }
        }

        // Calculate total length
        let total_len: usize = styled_spans.iter().map(|s| s.text.chars().count()).sum();
        if total_len == 0 {
            return Line::default();
        }

        let start = scroll_offset % total_len;
        let end = start + width;

        // Build visible spans by tracking character positions
        let mut visible_spans: Vec<Span<'static>> = Vec::new();
        let mut char_pos = 0;

        for span in styled_spans.iter().cycle() {
            let span_len = span.text.chars().count();
            let _span_start = char_pos;
            let span_end = char_pos + span_len;

            if span_end <= start {
                char_pos = span_end;
                continue;
            }

            if char_pos >= end {
                break;
            }

            // Calculate overlap with visible range
            let vis_start = if char_pos < start { start } else { char_pos };
            let vis_end = span_end.min(end);

            if vis_start >= vis_end {
                char_pos = span_end;
                continue;
            }

            // Extract the visible portion of this span's text
            let skip_chars = vis_start - char_pos;
            let take_chars = vis_end - vis_start;

            let vis_text: String = span
                .text
                .chars()
                .skip(skip_chars)
                .take(take_chars)
                .collect();

            let span = if let Some(color) = span.color {
                Span::styled(vis_text, Style::default().fg(color))
            } else {
                Span::raw(vis_text)
            };
            visible_spans.push(span);

            char_pos = span_end;

            if char_pos >= end {
                break;
            }
        }

        Line::from(visible_spans)
    }
}

fn format_price(price: f64) -> String {
    if price >= 10000.0 {
        format!("{:.0}", price)
    } else if price >= 1000.0 {
        format!("{:.1}", price)
    } else {
        format!("{:.2}", price)
    }
}

fn fetch_all_changes(
    client: &reqwest::blocking::Client,
    symbol: &str,
    show_hourly: bool,
) -> anyhow::Result<MarketItem> {
    let name = get_name_for_symbol(symbol);

    let (price, change_1h, change_24h, change_1w, change_1m) =
        fetch_intraday_and_monthly(client, symbol)?;

    Ok(MarketItem::new(
        symbol,
        name,
        price,
        change_1h,
        change_24h,
        change_1w,
        change_1m,
        show_hourly,
    ))
}

fn fetch_intraday_and_monthly(
    client: &reqwest::blocking::Client,
    symbol: &str,
) -> anyhow::Result<(f64, f64, f64, f64, f64)> {
    let url_day = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1h&range=1d",
        symbol
    );
    let url_month = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?interval=1d&range=1mo",
        symbol
    );

    let resp_day = client
        .get(&url_day)
        .header("User-Agent", "Mozilla/5.0")
        .send()?;
    let resp_month = client
        .get(&url_month)
        .header("User-Agent", "Mozilla/5.0")
        .send()?;

    if !resp_day.status().is_success() || !resp_month.status().is_success() {
        return Err(anyhow::anyhow!("API request failed"));
    }

    let json_day: serde_json::Value = resp_day.json()?;
    let json_month: serde_json::Value = resp_month.json()?;

    let result_day = json_day["chart"]["result"][0].clone();
    let result_month = json_month["chart"]["result"][0].clone();

    let price: f64 = result_day["meta"]["regularMarketPrice"]
        .as_f64()
        .or_else(|| result_month["meta"]["regularMarketPrice"].as_f64())
        .unwrap_or(0.0);

    let timestamps_day: Vec<f64> = result_day["timestamp"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    let quotes_day: Vec<f64> = result_day["indicators"]["quote"][0]["close"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    let timestamps_month: Vec<f64> = result_month["timestamp"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    let quotes_month: Vec<f64> = result_month["indicators"]["quote"][0]["close"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    let change_1h = calculate_1h_change(&timestamps_day, &quotes_day, price);
    let change_24h = calculate_24h_change(&timestamps_month, &quotes_month, price);
    let (change_1w, change_1m) =
        calculate_weekly_monthly_change(&timestamps_month, &quotes_month, price);

    Ok((price, change_1h, change_24h, change_1w, change_1m))
}

fn calculate_1h_change(timestamps: &[f64], quotes: &[f64], current_price: f64) -> f64 {
    if timestamps.is_empty() || quotes.is_empty() {
        return 0.0;
    }

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;

    let one_hour_ago = now_secs - 3600.0;

    let min_len = timestamps.len().min(quotes.len());

    for i in 0..min_len {
        if timestamps[i] >= one_hour_ago && i > 0 {
            let old_price = quotes[i - 1];
            if old_price > 0.0 {
                return ((current_price - old_price) / old_price) * 100.0;
            }
        }
    }

    if min_len >= 2 {
        let old_price = quotes[min_len - 2];
        if old_price > 0.0 {
            return ((current_price - old_price) / old_price) * 100.0;
        }
    }

    0.0
}

fn calculate_24h_change(timestamps: &[f64], quotes: &[f64], current_price: f64) -> f64 {
    if timestamps.is_empty() || quotes.is_empty() {
        return 0.0;
    }

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;

    let one_day_ago = now_secs - 86400.0;

    let min_len = timestamps.len().min(quotes.len());

    for i in 0..min_len {
        if timestamps[i] >= one_day_ago && i > 0 {
            let old_price = quotes[i - 1];
            if old_price > 0.0 {
                return ((current_price - old_price) / old_price) * 100.0;
            }
        }
    }

    if min_len >= 2 {
        let old_price = quotes[0];
        if old_price > 0.0 {
            return ((current_price - old_price) / old_price) * 100.0;
        }
    }

    0.0
}

fn calculate_weekly_monthly_change(
    timestamps: &[f64],
    quotes: &[f64],
    current_price: f64,
) -> (f64, f64) {
    if timestamps.is_empty() || quotes.is_empty() {
        return (0.0, 0.0);
    }

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as f64;

    let one_week_ago = now_secs - 604800.0;
    let one_month_ago = now_secs - 2592000.0;

    let mut week_price = current_price;
    let mut month_price = current_price;

    let min_len = timestamps.len().min(quotes.len());

    for i in 0..min_len {
        if timestamps[i] >= one_week_ago && i > 0 {
            week_price = quotes[i - 1];
            break;
        }
    }

    if week_price == current_price && min_len >= 7 {
        week_price = quotes[min_len - 7];
    }

    for i in 0..min_len {
        if timestamps[i] >= one_month_ago && i > 0 {
            month_price = quotes[i - 1];
            break;
        }
    }

    if month_price == current_price && !quotes.is_empty() {
        month_price = quotes[0];
    }

    let change_1w = if week_price > 0.0 {
        ((current_price - week_price) / week_price) * 100.0
    } else {
        0.0
    };

    let change_1m = if month_price > 0.0 {
        ((current_price - month_price) / month_price) * 100.0
    } else {
        0.0
    };

    (change_1w, change_1m)
}

fn get_name_for_symbol(symbol: &str) -> &'static str {
    match symbol {
        "GC=F" => "Gold",
        "SI=F" => "Silver",
        "CL=F" => "WTI Oil",
        "BZ=F" => "Brent",
        "NG=F" => "Nat Gas",
        "BTC-USD" => "Bitcoin",
        "ETH-USD" => "Ethereum",
        "^GSPC" => "S&P 500",
        "^DJI" => "Dow Jones",
        "^TNX" => "10Y Treasury",
        "^TYX" => "30Y Treasury",
        "EURUSD=X" => "USD/EUR",
        "USDJPY=X" => "USD/JPY",
        "USDCNY=X" => "USD/CNY",
        "USDRUB=X" => "USD/RUB",
        "USDINR=X" => "USD/INR",
        _ => "Unknown",
    }
}
