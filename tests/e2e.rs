use expectrl::{Error, Expect};
use std::time::Duration;

/// Helper function to spawn the app with fixture data
fn spawn_app() -> Result<impl Expect, Error> {
    // First ensure the binary is built
    std::process::Command::new("cargo")
        .args(["build"])
        .env("SITMON_USE_FIXTURES", "1")
        .output()
        .expect("Failed to build app");

    // Spawn the binary directly
    let session = expectrl::spawn("./target/debug/sitmon_cli")?;

    // Give the app time to start and load data
    std::thread::sleep(Duration::from_millis(500));
    Ok(session)
}

/// Kill the process cleanly
fn cleanup<E: Expect>(mut session: E) {
    let _ = session.send("q");
    std::thread::sleep(Duration::from_millis(100));
}

#[test]
fn test_navigation() -> Result<(), Error> {
    let mut app = spawn_app()?;

    // Wait for app to render with "Feed" visible
    app.expect("Feed")?;

    // Press 'j' to select first event
    app.send("j")?;
    std::thread::sleep(Duration::from_millis(100));

    // Press 'j' multiple times to navigate down
    for _ in 0..5 {
        app.send("j")?;
        std::thread::sleep(Duration::from_millis(50));
    }

    // Press 'k' to navigate back up
    for _ in 0..3 {
        app.send("k")?;
        std::thread::sleep(Duration::from_millis(50));
    }

    // Verify app is still responsive by checking Feed is still there
    app.expect("Feed")?;

    cleanup(app);
    Ok(())
}

#[test]
fn test_three_columns() -> Result<(), Error> {
    let mut app = spawn_app()?;

    // Wait for app to render
    app.expect("Feed")?;

    // Check that all three columns are visible
    app.expect("Warships")?;
    app.expect("World Leaders")?;

    cleanup(app);
    Ok(())
}
