use criterion::{criterion_group, criterion_main, Criterion};
use ratatui::{backend::TestBackend, layout::Rect, Terminal};
use std::hint::black_box;

use sitmon_cli::{
    app::App,
    data::{region_presets, FixtureProvider},
    ui,
};

/// Create app with fixture data for SWANEA region
fn create_swanea_app() -> App {
    // Set fixture environment variable
    unsafe {
        std::env::set_var("SITMON_USE_FIXTURES", "1");
    }
    let provider = FixtureProvider::from_default_fixtures();
    let mut app = App::new(region_presets(), &provider);

    // Set to SWANEA region (default)
    app.region_idx = app
        .regions
        .iter()
        .position(|r| r.name == "SWANEA (SW Asia, NE Africa)")
        .unwrap_or(0);

    app
}

/// Benchmark full UI render (feed + map + status)
fn bench_full_ui_render(c: &mut Criterion) {
    c.bench_function("full_ui_render_swanea", |b| {
        b.iter(|| {
            let app = create_swanea_app();
            let backend = TestBackend::new(160, 60);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| ui::draw(f, black_box(&mut app.clone())))
                .unwrap();
        })
    });
}

/// Benchmark feed rendering only
fn bench_feed_render(c: &mut Criterion) {
    let app = create_swanea_app();

    c.bench_function("feed_render_300_events", |b| {
        b.iter(|| {
            // Simulate rendering feed panel
            let objects = black_box(app.visible_objects());
            // Force evaluation
            black_box(objects.len());
        })
    });
}

/// Benchmark map rendering
fn bench_map_render(c: &mut Criterion) {
    c.bench_function("map_render_swanea_mercator", |b| {
        b.iter(|| {
            let app = create_swanea_app();
            let backend = TestBackend::new(100, 50);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|f| {
                    // Just render map portion
                    let area = Rect::new(0, 0, 100, 50);
                    let objects = app.visible_objects();
                    let selected = app.selected_object();
                    ui::render_map(
                        f,
                        area,
                        &app,
                        objects,
                        selected.map(|o| o.id.as_str()),
                        None,
                    );
                })
                .unwrap();
        })
    });
}

/// Benchmark land rasterization
fn bench_land_rasterization(c: &mut Criterion) {
    let mut app = create_swanea_app();

    c.bench_function("land_rasterization_cached", |b| {
        b.iter(|| {
            // This should use cache after first call
            let dots = app.get_land_dots(100, 40);
            black_box(dots.len());
        })
    });
}

/// Benchmark scroll offset calculation
fn bench_scroll_calculation(c: &mut Criterion) {
    let app = create_swanea_app();
    let objects = app.visible_objects();

    c.bench_function("scroll_offset_calculation", |b| {
        b.iter(|| {
            let offset = sitmon_cli::ui::calculate_scroll_offset(
                black_box(50),
                objects,
                black_box(40),
                black_box(80),
            );
            black_box(offset);
        })
    });
}

/// Stress test - render 1000 frames and measure FPS
fn stress_test_60fps(c: &mut Criterion) {
    c.bench_function("stress_1000_frames_fps", |b| {
        b.iter_custom(|iters| {
            let app = create_swanea_app();
            let backend = TestBackend::new(160, 60);
            let mut terminal = Terminal::new(backend).unwrap();

            let start = std::time::Instant::now();

            for _ in 0..iters {
                let mut app_clone = app.clone();
                terminal.draw(|f| ui::draw(f, &mut app_clone)).unwrap();
            }

            start.elapsed()
        });
    });
}

criterion_group!(
    benches,
    bench_full_ui_render,
    bench_feed_render,
    bench_map_render,
    bench_land_rasterization,
    bench_scroll_calculation,
    stress_test_60fps
);
criterion_main!(benches);
