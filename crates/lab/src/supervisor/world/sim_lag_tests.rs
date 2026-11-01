//! The `Sim` input-lag and coasting physics. Controller behaviour on top of
//! it is tested with the movement tools, not here.

use super::io::WorldIo;
use super::sim::Sim;

/// The pawn walks 5 UU per 10 ms step at the default 500 UU/s.
const STEP_UU: f64 = 5.0;

#[tokio::test]
async fn default_lag_changes_nothing() {
    let mut sim = Sim::live();
    assert_eq!(sim.lag.input_latency_ms, 0);
    assert_eq!(sim.lag.coast_s, 0.0);
    sim.key("W", true).await.unwrap();
    sim.sleep(100).await;
    assert!((sim.pos.x - 50.0).abs() < 1e-6, "moved {}", sim.pos.x);
    sim.key("W", false).await.unwrap();
    sim.sleep(100).await;
    assert!((sim.pos.x - 50.0).abs() < 1e-6, "moved on after W up");
}

#[tokio::test]
async fn latency_delays_the_press() {
    let mut sim = Sim::live();
    sim.lag.input_latency_ms = 150;
    sim.key("W", true).await.unwrap();
    assert!(
        !sim.keys.contains_key("W"),
        "press applied before its latency"
    );
    sim.sleep(100).await;
    assert!(
        sim.pos.x.abs() < 1e-6,
        "moved {} before the press landed",
        sim.pos.x
    );
    sim.sleep(100).await;
    // The press lands at 150 ms; the steps ending at 150..=200 ms walk: 6 steps.
    assert!(
        (sim.pos.x - 6.0 * STEP_UU).abs() < 1e-6,
        "moved {}",
        sim.pos.x
    );
}

#[tokio::test]
async fn latency_delays_the_release_too() {
    let mut sim = Sim::live();
    sim.lag.input_latency_ms = 150;
    sim.key("W", true).await.unwrap();
    sim.sleep(500).await;
    sim.key("W", false).await.unwrap();
    let before = sim.pos.x;
    sim.sleep(100).await;
    assert!(
        (sim.pos.x - before - 10.0 * STEP_UU).abs() < 1e-6,
        "stopped early: moved {}",
        sim.pos.x - before
    );
}

#[tokio::test]
async fn coast_after_release() {
    let mut sim = Sim::live();
    sim.lag.coast_s = 0.3;
    sim.key("W", true).await.unwrap();
    sim.sleep(1000).await;
    sim.key("W", false).await.unwrap();
    let released_at = sim.pos.x;
    sim.sleep(1000).await;
    assert!(
        (sim.pos.x - released_at - 150.0).abs() < 1e-6,
        "coasted {}",
        sim.pos.x - released_at
    );
}

#[tokio::test]
async fn pressing_again_ends_the_coast() {
    let mut sim = Sim::live();
    sim.lag.coast_s = 0.3;
    sim.key("W", true).await.unwrap();
    sim.sleep(1000).await;
    sim.key("W", false).await.unwrap();
    let at_release = sim.pos.x;
    sim.sleep(100).await;
    assert!(
        (sim.pos.x - at_release - 50.0).abs() < 1e-6,
        "first coast ran {}",
        sim.pos.x - at_release
    );
    sim.key("W", true).await.unwrap();
    assert!(!sim.lag.coasting(sim.now), "coast survived the press");
    sim.key("W", false).await.unwrap();
    let second_release = sim.pos.x;
    sim.sleep(1000).await;
    assert!(
        (sim.pos.x - second_release - 150.0).abs() < 1e-6,
        "second coast {}",
        sim.pos.x - second_release
    );
}
