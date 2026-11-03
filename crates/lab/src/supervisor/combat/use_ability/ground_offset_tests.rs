//! The offset form of the ground point: resolved from the live avatar and
//! camera, retried when the hover rejects it, and reported.

use std::f64::consts::FRAC_PI_2;

use serde_json::{json, Value};

use super::ground::{confirm, plan, ClickFail, GroundArg, GroundClicker};
use crate::supervisor::world::geometry::{horizontal_m, Vec3};
use crate::supervisor::world::{PointArg, Space};

/// A client whose avatar is at `player` with the camera at `yaw`, and whose
/// hover refuses (naming a click step) the points `reject` picks, as the
/// avatar's own body does.
struct Fake {
    player: Vec3,
    yaw: f64,
    view_error: Option<String>,
    reject: Box<dyn Fn(Vec3) -> Option<&'static str>>,
    clicks: Vec<PointArg>,
    rotates: Vec<bool>,
}

impl Fake {
    fn new(
        player: Vec3,
        yaw: f64,
        reject: impl Fn(Vec3) -> Option<&'static str> + 'static,
    ) -> Self {
        Self {
            player,
            yaw,
            view_error: None,
            reject: Box::new(reject),
            clicks: vec![],
            rotates: vec![],
        }
    }
}

impl GroundClicker for Fake {
    async fn view(&mut self) -> Result<(Vec3, f64), String> {
        match &self.view_error {
            Some(e) => Err(e.clone()),
            None => Ok((self.player, self.yaw)),
        }
    }

    async fn click(&mut self, p: PointArg, rotate: bool) -> Result<Value, ClickFail> {
        self.clicks.push(p);
        self.rotates.push(rotate);
        match (self.reject)(Vec3::new(p.x, p.y, p.z)) {
            Some(step) => Err(ClickFail {
                point: p,
                step: step.into(),
                message: "your own avatar covers the point".into(),
            }),
            None => Ok(json!({ "native_level": "real_input" })),
        }
    }
}

fn offset(distance_m: f64, bearing_deg: f64) -> GroundArg {
    GroundArg {
        distance_m: Some(distance_m),
        bearing_deg: Some(bearing_deg),
        ..GroundArg::default()
    }
}

#[test]
fn the_first_offset_is_distance_metres_along_the_camera_yaw() {
    // Camera facing +Y: 8 m ahead is +800 units of Y, wherever the avatar is.
    let player = Vec3::new(-84000.0, 25000.0, 700.0);
    let c = plan(player, FRAC_PI_2, &GroundArg::default()).unwrap()[0];
    assert!((c.point.x - player.x).abs() < 1e-6, "{c:?}");
    assert!((c.point.y - (player.y + 800.0)).abs() < 1e-6, "{c:?}");
    assert!((horizontal_m(player, c.point) - 8.0).abs() < 1e-9);
    assert!(
        (c.point.z - player.z).abs() < 1e-9,
        "the pawn Location is the feet, which is ground level"
    );
}

#[test]
fn the_point_follows_the_avatar_not_a_fixed_place() {
    let a = plan(Vec3::new(0.0, 0.0, 0.0), 0.0, &offset(8.0, 30.0)).unwrap()[0];
    let b = plan(Vec3::new(5000.0, -3000.0, 0.0), 0.0, &offset(8.0, 30.0)).unwrap()[0];
    assert!((b.point.x - a.point.x - 5000.0).abs() < 1e-6);
    assert!((b.point.y - a.point.y + 3000.0).abs() < 1e-6);
}

#[test]
fn the_retries_are_the_documented_rings_and_sides_without_repeats() {
    let list = plan(Vec3::new(0.0, 0.0, 0.0), 0.0, &GroundArg::default()).unwrap();
    let got: Vec<(f64, f64)> = list.iter().map(|c| (c.distance_m, c.bearing_deg)).collect();
    assert_eq!(
        got,
        vec![
            (8.0, 0.0),
            (8.0, 30.0),
            (8.0, -30.0),
            (6.0, 30.0),
            (6.0, -30.0),
            (10.0, 30.0)
        ]
    );
    // A first offset equal to a retry ring is not tried twice.
    let list = plan(Vec3::new(0.0, 0.0, 0.0), 0.0, &offset(6.0, 0.0)).unwrap();
    for (i, a) in list.iter().enumerate() {
        for b in &list[i + 1..] {
            assert!(
                (a.distance_m, a.bearing_deg) != (b.distance_m, b.bearing_deg),
                "{list:?}"
            );
        }
    }
}

#[test]
fn a_bad_offset_is_refused_before_any_click() {
    let o = Vec3::new(0.0, 0.0, 0.0);
    assert!(plan(o, 0.0, &offset(0.0, 0.0)).is_err());
    assert!(plan(o, 0.0, &offset(99.0, 0.0)).is_err());
    assert!(plan(o, 0.0, &offset(8.0, f64::NAN)).is_err());
    let partial = GroundArg {
        x: Some(1.0),
        ..GroundArg::default()
    };
    assert!(partial.absolute().is_err());
}

#[tokio::test]
async fn a_covered_first_offset_is_retried_at_the_next_and_the_point_used_is_reported() {
    // Camera facing +X from behind the avatar: the straight-ahead strip is
    // covered by its body.
    let player = Vec3::new(1000.0, 2000.0, 300.0);
    let mut fake = Fake::new(player, 0.0, move |p| {
        ((p.y - player.y).abs() < 100.0).then_some("hover")
    });
    let out = confirm(&mut fake, GroundArg::default()).await.unwrap();
    assert_eq!(
        fake.clicks.len(),
        2,
        "straight ahead refused, +30 deg taken"
    );
    assert_eq!(out["bearing_deg"], 30.0);
    assert_eq!(out["distance_m"], 8.0);
    assert_eq!(out["refused"].as_array().unwrap().len(), 1);
    assert_eq!(out["refused"][0]["step"], "hover");
    assert_eq!(fake.rotates, vec![false, false]);
    assert_eq!(out["point"]["space"], "client");
    assert_eq!(out["point"]["x"], fake.clicks[1].x);
}

#[tokio::test]
async fn an_off_screen_offset_is_retried_too() {
    let mut fake = Fake::new(Vec3::new(0.0, 0.0, 0.0), 0.0, |p| {
        (p.y >= 0.0).then_some("on_screen")
    });
    let out = confirm(&mut fake, GroundArg::default()).await.unwrap();
    assert_eq!(out["bearing_deg"], -30.0, "{out}");
    assert_eq!(out["refused"][1]["step"], "on_screen");
}

#[tokio::test]
async fn every_offset_refused_fails_naming_them_all() {
    let mut fake = Fake::new(Vec3::new(0.0, 0.0, 0.0), 0.0, |_| Some("hover"));
    let e = confirm(&mut fake, GroundArg::default()).await.unwrap_err();
    assert_eq!(fake.clicks.len(), 6);
    assert!(
        e.contains("could be clicked") && e.contains("bearing_deg"),
        "{e}"
    );
}

#[tokio::test]
async fn a_failure_that_is_not_the_hover_stops_at_once() {
    let mut fake = Fake::new(Vec3::new(0.0, 0.0, 0.0), 0.0, |_| Some("click"));
    let e = confirm(&mut fake, GroundArg::default()).await.unwrap_err();
    assert_eq!(fake.clicks.len(), 1);
    assert!(e.contains("failed at click"), "{e}");
}

#[tokio::test]
async fn an_absolute_point_is_clicked_as_given_and_never_replaced() {
    let abs = GroundArg {
        x: Some(-87000.0),
        y: Some(24600.0),
        z: Some(663.0),
        space: Some(Space::Client),
        ..GroundArg::default()
    };
    let mut fake = Fake::new(Vec3::new(0.0, 0.0, 0.0), 0.0, |_| None);
    let out = confirm(&mut fake, abs).await.unwrap();
    assert_eq!(fake.clicks.len(), 1);
    assert_eq!(fake.clicks[0].x, -87000.0);
    assert_eq!(fake.rotates, vec![true]);
    assert_eq!(out["point"]["x"], -87000.0);

    let mut covered = Fake::new(Vec3::new(0.0, 0.0, 0.0), 0.0, |_| Some("hover"));
    assert!(confirm(&mut covered, abs).await.is_err());
    assert_eq!(
        covered.clicks.len(),
        1,
        "no alternatives for an explicit point"
    );
}

#[tokio::test]
async fn an_unreadable_avatar_position_fails_the_offset_form() {
    let mut fake = Fake::new(Vec3::new(0.0, 0.0, 0.0), 0.0, |_| None);
    fake.view_error = Some("the avatar has no pose".into());
    let e = confirm(&mut fake, GroundArg::default()).await.unwrap_err();
    assert!(e.contains("no pose") && fake.clicks.is_empty(), "{e}");
}
