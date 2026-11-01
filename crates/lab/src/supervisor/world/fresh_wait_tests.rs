use super::fresh_wait::wait_fresh;
use super::sim::Sim;
use crate::proof::fresh::Freshness;

#[tokio::test]
async fn wait_fresh_waits_out_a_stale_frame() {
    let mut sim = Sim::new();
    sim.frames = Some(10);
    let (f, frame) = wait_fresh(&mut sim, Some(10)).await;
    assert_eq!(f, Freshness::Fresh);
    assert!(sim.now >= 50, "slept at least once");
    assert!(frame.unwrap() >= 12);
}

#[tokio::test]
async fn wait_fresh_gives_up_after_three() {
    let mut sim = Sim::new();
    sim.frames = Some(10);
    // The action frame is far ahead of anything three sleeps can reach.
    let (f, frame) = wait_fresh(&mut sim, Some(1000)).await;
    assert_eq!(f, Freshness::Stale);
    assert_eq!(sim.now, 150, "exactly 3 sleeps of 50 ms");
    assert_eq!(frame, sim.frames);
}

#[tokio::test]
async fn unstamped_without_frames() {
    let mut sim = Sim::new();
    let (f, frame) = wait_fresh(&mut sim, Some(10)).await;
    assert_eq!((f, frame), (Freshness::Unstamped, None));
    assert_eq!(sim.now, 0, "no sleep");
}
