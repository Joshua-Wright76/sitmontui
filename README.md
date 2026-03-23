# sitmon-cli

Rust TUI situation monitor with a Braille world/regional map, left-side feed, keyboard-only controls, and static mock data adapters shaped for ACLED + VesselFinder.

## Run

```bash
cargo run
```

Tested for flexible terminal sizes like `100x30` and `160x45`.

## Current data mode

- Live provider support for ACLED incidents and VesselFinder ships
- Automatic fallback to mock data if live credentials/config are missing or invalid
- `.env.example` includes `ACLED_API_EMAIL`, `ACLED_API_KEY`, `ACLED_REFRESH_KEY`, `VESSELFINDER_API_KEY`, `VESSELFINDER_API_URL`

## Layout

- Left pane: feed (incidents and ships in viewport)
- Right pane: Braille map raster + symbols
- Landmass source: Natural Earth simplified 110m (`assets/ne_110m_land.geojson`)
- Bottom pane: status, filters, and key help

## Keybindings

- `q` or `Ctrl+C`: quit
- `]` / `[`: next/previous region preset (`r` / `R` also work)
- `z` / `x`: zoom in/out (`regional`, `2x`, `4x`) (`+` / `-` also work)
- `0`: reset zoom to regional
- `m`: cycle map contrast mode (`normal`, `high`, `transparent-safe`)
- `t`: open/close layer popup
- layer popup: `j`/`k` move, `space` toggle, `a` toggle all, `d` defaults, `Esc` close
- `tab`: switch active pane (`feed` or `map`)
- `w` `a` `s` `d`: pan map (works from any pane)
- `j` / `k`: move selection in feed when feed pane is active
- `h` `j` `k` `l` or arrow keys: pan viewport when map pane is active
- `n` / `p`: next/previous object selection
- `c`: center on selected object
- `1` `2` `3` `4`: toggle severity (low, medium, high, critical)
- `g`: refresh snapshot manually

## Symbols and palette

- Incidents: `· ! ▲ ◆` (low to critical)
- Ships: `⛴` underway, `◇` anchored
- Cities: `★` (major city)
- Capitals: country flag emoji with `✪` fallback
- Selected object: `⬢`
- Rivers: `≈` overlay
- Mountain ranges: `∧` overlay
- Color-blind-safe default palette (Okabe-Ito inspired)
- Default mode is `transparent-safe` for readability on transparent terminals

## Regional projections

- Most regions use equirectangular projection
- `Arctic Circle` uses north-polar projection

## Region presets

- North America
- Central America/Caribbean
- Northern South America
- Southern Cone South America
- Oceania
- North/West Africa
- Southern/Eastern/Central Africa
- Europe
- SWANEA (SW Asia, NE Africa)
- Southeast Asia
- South/Central Asia
- East Asia
- Arctic Circle
- North Pacific Ocean
