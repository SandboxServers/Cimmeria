//! SS-D3: every end path clears the PvP flag (CAT-M-15), and every travel
//! site calls the duel hook.

use std::time::Duration;

use tokio::sync::mpsc;

use cimmeria_common::Vector3;
use cimmeria_entity::stats::HEALTH;
use cimmeria_wire::cell::client_methods::duel::build_pvp_flag;

use super::end_paths::{drain_duel, ended_row};
use super::engage::{aoi_mgr, engage, to_witnesses, DUEL_CLEAR, PVP_FLAG};
use super::*;
use crate::cell::duel::limits::{ENGAGED_LIMIT, RANGE_GRACE};
use crate::cell::duel::tick::run_at;
use crate::cell::duel::{end_engaged, EndReason};
use crate::test_support::LogCapture;

/// One way an engaged duel can end.
#[derive(Debug, Clone, Copy)]
enum Path {
    Forfeit,
    PartnerClamp,
    ThirdPartyDeath,
    Disconnect,
    Travel,
    Range,
    EngagedLimit,
    GmAborted,
    /// B's entity is removed by a path with no hook; the sweep ends it.
    GoneWithoutHook,
}

impl Path {
    const ALL: [Path; 9] = [
        Path::Forfeit,
        Path::PartnerClamp,
        Path::ThirdPartyDeath,
        Path::Disconnect,
        Path::Travel,
        Path::Range,
        Path::EngagedLimit,
        Path::GmAborted,
        Path::GoneWithoutHook,
    ];

    /// The `reason` on the `duel.ended` row.
    fn reason(self) -> &'static str {
        match self {
            Path::Forfeit => "forfeit",
            Path::PartnerClamp | Path::ThirdPartyDeath => "health",
            Path::Disconnect | Path::GoneWithoutHook => "connection",
            Path::Travel => "teleport",
            Path::Range => "range",
            Path::EngagedLimit => "engaged_limit",
            Path::GmAborted => "gm_aborted",
        }
    }

    /// Whether B's engaged entity is still there to clear. Only a bare
    /// removal leaves nothing to clear on B's side.
    fn b_cleared(self) -> bool {
        !matches!(self, Path::GoneWithoutHook)
    }
}

async fn take(
    path: Path,
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    engaged_at: std::time::Instant,
) {
    match path {
        Path::Forfeit => crate::cell::duel::forfeit::handle(B_EID, tx, mgr).await,
        Path::PartnerClamp => {
            let hp = mgr
                .get_entity_mut(B_EID)
                .unwrap()
                .stats
                .get_mut(HEALTH)
                .unwrap();
            hp.update(hp.min, -5, hp.max);
            let hit = crate::cell::duel::clamp_partner_lethal(mgr, A_EID, B_EID, "test")
                .expect("A's lethal hit on B is clamped");
            crate::cell::duel::finish_clamped(tx, mgr, hit).await;
        }
        Path::ThirdPartyDeath => crate::cell::duel::on_death(tx, mgr, B_EID).await,
        Path::Disconnect => mgr.disconnect_entity(B_EID, tx).await,
        Path::Travel => crate::cell::duel::on_travel(tx, mgr, B_EID).await,
        Path::Range => {
            mgr.get_entity_mut(B_EID).unwrap().position = Vector3::new(60.0, 0.0, 0.0);
            run_at(tx, mgr, engaged_at).await;
            run_at(tx, mgr, engaged_at + RANGE_GRACE).await;
        }
        Path::EngagedLimit => run_at(tx, mgr, engaged_at + ENGAGED_LIMIT).await,
        Path::GmAborted => {
            let id = mgr.duels.duel_of(A_PID).unwrap().duel_id;
            end_engaged(tx, mgr, id, EndReason::GmAborted).await;
        }
        Path::GoneWithoutHook => {
            mgr.destroy_entity(B_EID);
            run_at(tx, mgr, engaged_at + Duration::from_millis(100)).await;
        }
    }
}

/// **CAT-M-15, type 8.** Every end path, one row of the table each, ends
/// the duel through the one clear: each duelist whose engaged entity is
/// still there gets its own flag set to 0, every witness gets the same,
/// both get 153, and the registry forgets both players. A path that skipped
/// `end_engaged` (removed the duel itself, or never ended it) fails its row.
#[tokio::test]
async fn every_end_path_clears_pvp_flag() {
    let off = build_pvp_flag(false);
    let mut failures = Vec::new();
    for path in Path::ALL {
        let capture = LogCapture::install();
        let mut mgr = aoi_mgr();
        let (tx, mut rx) = mpsc::channel(512);
        let engaged_at = engage(&mut mgr, &tx, &mut rx).await;
        drain(&mut rx);

        take(path, &mut mgr, &tx, engaged_at).await;
        let sent = drain_duel(&mut rx);

        let mut check = |ok: bool, what: &str| {
            if !ok {
                failures.push(format!("{path:?}: {what}"));
            }
        };
        let mut sides = vec![(A_EID, [B_EID, C_EID])];
        if path.b_cleared() {
            sides.push((B_EID, [A_EID, C_EID]));
        }
        for (me, witnesses) in sides {
            check(
                own(&sent, me, PVP_FLAG) == vec![off.clone()],
                &format!("{me}'s own flag not cleared"),
            );
            let seen: Vec<u32> = to_witnesses(&sent, me, PVP_FLAG)
                .into_iter()
                .filter(|(_, a)| *a == off)
                .map(|(w, _)| w)
                .collect();
            for w in witnesses {
                // A witness who is gone (B after a hookless removal) is not
                // owed anything.
                if mgr.get_entity(w).is_some() || path.b_cleared() {
                    check(
                        seen.contains(&w),
                        &format!("{me}'s flag not cleared for witness {w}"),
                    );
                }
            }
            check(
                own(&sent, me, DUEL_CLEAR).len() == 1,
                &format!("{me} got no 153"),
            );
        }
        check(
            !mgr.duels.is_busy(A_PID) && !mgr.duels.is_busy(B_PID),
            "the registry still holds a duelist",
        );
        check(
            !mgr.duels.can_harm(A_PID, B_PID),
            "the pair can still harm each other",
        );
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", "duel.ended"))
            .collect();
        check(
            rows.len() == 1 && rows[0].has_field("reason", path.reason()),
            &format!("expected one duel.ended reason={}", path.reason()),
        );
        if rows.len() == 1 {
            let _ = ended_row(&capture);
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Every cell path that sends `TeleportPlayer` or `GateTravel` calls
/// `duel::on_travel`, so a travelling duelist loses at once with
/// `EDUEL_DEFEAT_Teleport` instead of leaving the partner flagged until the
/// sweep. The movement validator's snap-back is exempt: it puts a player
/// back where it already was.
#[test]
fn every_travel_site_ends_the_duel() {
    const EXEMPT: &[&str] = &["cell/src/cell/service/base_messages/movement.rs"];
    let crates_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut missing = Vec::new();
    let mut seen = 0usize;
    for krate in [
        "cell",
        "cell-combat",
        "cell-content",
        "cell-interactions",
        "cell-console",
        "cell-methods",
        "cell-world",
    ] {
        let mut stack = vec![crates_dir.join(krate).join("src")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    if path
                        .file_name()
                        .is_some_and(|n| !n.to_string_lossy().contains("test"))
                    {
                        stack.push(path);
                    }
                    continue;
                }
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if !name.ends_with(".rs") || name.contains("test") {
                    continue;
                }
                let text = std::fs::read_to_string(&path)
                    .unwrap()
                    .replace("\r\n", "\n");
                let code = text
                    .find("#[cfg(test)]\nmod tests {")
                    .map_or(text.as_str(), |i| &text[..i]);
                let rel = path
                    .strip_prefix(&crates_dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let sends = code.matches("CellToBaseMsg::TeleportPlayer {").count()
                    + code.matches("CellToBaseMsg::GateTravel {").count();
                if sends == 0 || EXEMPT.contains(&rel.as_str()) {
                    continue;
                }
                seen += sends;
                let hooks = code.matches("duel::on_travel(").count();
                if hooks < sends {
                    missing.push(format!("{rel}: {sends} travel sends, {hooks} duel hooks"));
                }
            }
        }
    }
    assert!(
        seen >= 12,
        "the scan found too few travel sites ({seen}); it is not looking where they are"
    );
    assert!(missing.is_empty(), "{missing:#?}");
}
