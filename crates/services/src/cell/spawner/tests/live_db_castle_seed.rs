//! Live-DB guards for the Castle (World 8) story-actor seed rows authored by
//! packet CA05 — templates 168-173, `spawnlist` 238-246, point sets 2082-2085.
//!
//! Split out of `live_db_loaders.rs`, which these pushed past the 700-line hard
//! cap. The seam is real rather than cosmetic: `live_db_loaders.rs` guards the
//! *loader queries* (column renames, type drift, JOIN breakage — bugs that make
//! sqlx return `Err`), whereas everything here guards *seed content* that loads
//! perfectly well and is simply wrong. Sibling Castle/Harset seed packets should
//! land their content guards here too.
//!
//! What "wrong" has actually meant on this packet, and therefore what each guard
//! reproduces:
//!
//! * a story actor placed in the wrong room, or on the wrong floor of the right
//!   room (the comms pair sat at y = 66.79 while `Castle.CommsRoom` was at
//!   y = 55.20, and Warden Muelbach stood at Checkpoint Bravo when objective 2799
//!   says she is in the bunker *above* it);
//! * a region box authored flat — all four corners sharing one y — which is a
//!   zero-height volume containing only a player standing exactly on the floor
//!   plane, so its `enter_region` trigger effectively never fires;
//! * a `name_id` left NULL or pointing at the wrong recovered moniker, which ships
//!   a nameless or mislabelled NPC with no server-side error;
//! * a hostile row with no respawn timer, which deletes that mob from the shared
//!   world for everyone the first time anybody kills it.
//!
//! Every one of those shipped in some draft of this packet and every one of them
//! survived a fully green test run, which is the argument for this file existing.
//!
//! Note on the position guards: the server does **not** hit-test these boxes.
//! Region entry is client-reported — `fire_enter_region`'s own doc comment says it
//! fires "when the client crosses a Kismet trigger volume", and the box is pushed
//! to the client at player-init (`service/base_messages/player_init/mod.rs:369`).
//! So these are seed-*consistency* guards: they catch an actor and the box that is
//! supposed to contain it drifting apart, which no runtime check would ever
//! notice.
mod live_db {
    use crate::cell::spawner::*;
    use crate::test_support::require_db_or_skip;

    /// Every CA05 story actor as `(spawnlist.tag, template_id, name_id)`.
    ///
    /// One table drives the identity, template-binding and display-name guards so
    /// they cannot disagree about the cast. `name_id`s are recovered originals from
    /// `resources.texts` (see `worknotes/ca05.md` "Recovered display names") and are
    /// pinned to exact values, not merely asserted non-zero: a transposed digit
    /// still renders *a* name, just the wrong character's.
    const CA05_ACTORS: [(&str, i32, i32); 9] = [
        // 7066 DN_npc_mg_Zuritska_Castle_OLD        = 'Dr. Zuritska'
        ("Castle_Zuritska_Cell", 168, 7066),
        ("Castle_Zuritska_Comms", 168, 7066),
        // 6962 DN_MsMb_Castle_Romney_Uni_3          = 'NID Interrogator Romney'
        ("Castle_Romney", 169, 6962),
        // 6965 DN_Mb_Castle_Warden_Uni_4            = 'Warden Muelbach'
        ("Castle_Muelbach", 170, 6965),
        // 6966 DN_MB_Castle_NID_Officer_St_4-5      = 'NID Officer' (objective 2798's noun)
        ("Castle_BravoOfficer1", 171, 6966),
        ("Castle_BravoOfficer2", 171, 6966),
        ("Castle_BravoOfficer3", 171, 6966),
        // 7417 DN_Mb_Castle_NID_Guard_St_1-5        = 'NID Guard' (kept: objective 2794
        // calls him only "a guard", so there is no unique string to recover)
        ("Castle_SurrenderGuard", 172, 7417),
        // 7720 DN_Ob_D_HumanViewScreen_Castle_CommTerminal = 'Communications Terminal'
        ("Castle_CommsTerminal", 173, 7720),
    ];

    const CA05_POINT_SETS: [&str; 4] = [
        "Castle.InterrogationBlock",
        "Castle.CommsRoom",
        "Castle.CheckpointBravo",
        "Castle.CheckpointAlpha",
    ];

    /// `(tag, point_set)` pairs the seed claims contain each other.
    const ACTORS_INSIDE: [(&str, &str); 8] = [
        ("Castle_Zuritska_Cell", "Castle.InterrogationBlock"),
        ("Castle_Romney", "Castle.InterrogationBlock"),
        ("Castle_Zuritska_Comms", "Castle.CommsRoom"),
        ("Castle_CommsTerminal", "Castle.CommsRoom"),
        ("Castle_BravoOfficer1", "Castle.CheckpointBravo"),
        ("Castle_BravoOfficer2", "Castle.CheckpointBravo"),
        ("Castle_BravoOfficer3", "Castle.CheckpointBravo"),
        ("Castle_SurrenderGuard", "Castle.CheckpointBravo"),
    ];

    /// Downward slack allowed between an actor and its box's floor plane, in world
    /// units. Generous enough for uneven terrain (`Castle_BravoOfficer3` sits 1.0
    /// above the Checkpoint Bravo plane on a rise) but far tighter than the errors
    /// this is here to catch: the comms pair were 11.6 units off their room's floor
    /// and Muelbach 23 units off the bunker's. Upward slack needs no constant — the
    /// box carries its own ceiling.
    const FLOOR_TOLERANCE: f32 = 3.0;

    /// Smallest vertical extent a Castle region box may have. See
    /// [`castle_ca05_point_sets_are_four_corner_world8_boxes`] for why zero is a bug
    /// and not merely untidy.
    const MIN_BOX_HEIGHT: f32 = 1.0;

    /// Axis-aligned volume derived from a seeded `BoundingBox` point set the way the
    /// engine derives it: the point-wise min/max on each axis.
    struct Box3 {
        x: (f32, f32),
        z: (f32, f32),
        floor: f32,
        ceiling: f32,
    }

    impl Box3 {
        fn contains_xz(&self, x: f32, z: f32) -> bool {
            x >= self.x.0 && x <= self.x.1 && z >= self.z.0 && z <= self.z.1
        }

        /// Vertically inside the volume, allowing [`FLOOR_TOLERANCE`] below the floor
        /// plane for terrain slop. The ceiling is the box's own, not a constant.
        fn contains_y(&self, y: f32) -> bool {
            y >= self.floor - FLOOR_TOLERANCE && y <= self.ceiling
        }

        fn height(&self) -> f32 {
            self.ceiling - self.floor
        }
    }

    fn region_box(regions: &[RegionLoadData], name: &str) -> Box3 {
        let region = regions
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("point set {name} must be loaded by load_regions_from_db"));
        assert!(
            !region.points.is_empty(),
            "point set {name} has no points, cannot form a box"
        );
        let fold = |v: Vec<f32>| {
            v.iter()
                .fold((f32::MAX, f32::MIN), |(lo, hi), &n| (lo.min(n), hi.max(n)))
        };
        let (y_lo, y_hi) = fold(region.points.iter().map(|p| p[1]).collect());
        Box3 {
            x: fold(region.points.iter().map(|p| p[0]).collect()),
            z: fold(region.points.iter().map(|p| p[2]).collect()),
            floor: y_lo,
            ceiling: y_hi,
        }
    }

    fn spawn_by_tag<'a>(records: &'a [SpawnRecord], tag: &str) -> &'a SpawnRecord {
        records
            .iter()
            .find(|r| r.tag.as_deref() == Some(tag))
            .unwrap_or_else(|| panic!("tag {tag} must have a resources.spawnlist row"))
    }

    /// **CA05 regression guard** (work-packets.md CA05 acceptance): every story-actor
    /// tag resolves to exactly one World 8 spawn row, bound to the template the
    /// packet assigned it.
    ///
    /// `template_id` is pinned, not just the join's success: repointing a tag at
    /// another template keeps `template_name` non-empty and would sail past a
    /// laxer assertion while silently swapping the actor's body, faction and
    /// stats — e.g. Zuritska (168, faction 1) onto a hostile NID guard.
    #[tokio::test]
    async fn castle_ca05_story_actor_tags_resolve_to_exactly_one_spawn_row() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");

        for (tag, want_template, _) in CA05_ACTORS {
            let matches: Vec<&SpawnRecord> = records
                .iter()
                .filter(|r| r.tag.as_deref() == Some(tag))
                .collect();
            assert_eq!(
                matches.len(),
                1,
                "tag {tag} must resolve to exactly one resources.spawnlist row; found {}. \
                 A deleted row soft-locks the mission step that targets it; a duplicate \
                 makes which one the chain binds nondeterministic",
                matches.len()
            );
            let record = matches[0];
            assert_eq!(
                record.world_name, "Castle",
                "tag {tag} must be a World 8 (Castle) spawn, got world {}",
                record.world_name
            );
            assert_eq!(
                record.template_id, want_template,
                "tag {tag} must use template {want_template}, got {}. The template \
                 carries the actor's body, faction, name and speed — a repointed tag \
                 changes the character without changing anything the chains can see",
                record.template_id
            );
            assert!(
                !record.template_name.is_empty(),
                "tag {tag} spawn row's entity_templates join produced an empty \
                 template_name — the referenced template_id was deleted or renamed"
            );
        }
    }

    /// **CA05 regression guard**: every named CA05 actor resolves the exact recovered
    /// `name_id`, and that id exists in `resources.texts`.
    ///
    /// `name_id` is written raw onto the AoI create packet and only when it is
    /// `Some(n)` with `n != 0` (`mercury/aoi/create.rs:211-218`); the client resolves
    /// it against its own PAK string table. So a NULL or 0 column silently ships a
    /// nameless NPC — no error, no log — and a wrong-but-nonzero id silently ships
    /// the wrong character's name. Both shapes are caught here, which is why this
    /// asserts equality against [`CA05_ACTORS`] rather than `!= 0`.
    ///
    /// The `resources.texts` lookup is a second, independent check: it fails on an
    /// id that was mistyped into a value no shipped moniker uses. It cannot prove the
    /// *client* has the string (that lives in the PAK, not the DB), so treat it as a
    /// typo guard, not a rendering guarantee.
    #[tokio::test]
    async fn castle_ca05_story_actors_carry_their_recovered_name_ids() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");

        for (tag, _, want_name_id) in CA05_ACTORS {
            let record = spawn_by_tag(&records, tag);
            assert_eq!(
                record.name_id,
                Some(want_name_id),
                "tag {tag} (template {}) must carry name_id {want_name_id}; got {:?}. \
                 NULL/0 omits the name property from the AoI create packet entirely \
                 (the NPC appears unnamed); a different non-zero id renders another \
                 character's name",
                record.template_id,
                record.name_id
            );

            let moniker: Option<(String,)> =
                sqlx::query_as("SELECT moniker_name FROM resources.texts WHERE moniker_id = $1")
                    .bind(want_name_id)
                    .fetch_optional(&pool)
                    .await
                    .expect("resources.texts probe must succeed");
            assert!(
                moniker.is_some(),
                "name_id {want_name_id} (tag {tag}) has no resources.texts row — it is \
                 not a recovered moniker, so it is a typo. New ids cannot be minted: \
                 the client resolves name_id against its own PAK string table"
            );
        }
    }

    /// **CA05 regression guard**: every new Castle point set loads in World 8 with
    /// exactly four corner points **and a non-zero vertical extent**.
    ///
    /// Four is an engine invariant, not a style preference — `space_manager/mod.rs:74`:
    /// "After the cylinder→bbox workaround, all regions should have exactly 4 points."
    /// A three-point box is a degenerate triangle the client hit-tests wrongly, and a
    /// zero-point set never fires its `enter_region` trigger at all, soft-locking
    /// whichever mission step waits on it.
    ///
    /// The height assertion is the one that caught a live bug, so it is worth spelling
    /// out. An `AreaSet` volume is the point-wise min/max of its corners, so four
    /// corners sharing one y describe a box of zero height that contains only a player
    /// standing exactly on the floor plane. The shipped convention puts three corners
    /// on the floor and raises the fourth — which is literally what the engine's own
    /// cylinder→bbox workaround emits (`spawner/regions.rs`, whose comment insists the
    /// asymmetry is intentional: "do not 'normalize' by raising all four corners") —
    /// and every pre-existing box follows it: 2040 +6.31, 2041 +5.70, 2042 +16.31,
    /// 2043 +7.26, 2044 +7.61, 2049 +7.04, 2050 +8.07, 2051 +4.33.
    ///
    /// CA05's first draft authored all four boxes flat. Nothing failed: the seed loads,
    /// the loaders are happy, and the only symptom would have been mission 702 step
    /// 2402 and mission 704 step 2405 never firing in-client, with no error logged
    /// anywhere. Flattening any of these boxes again must fail here.
    #[tokio::test]
    async fn castle_ca05_point_sets_are_four_corner_world8_boxes() {
        let pool = require_db_or_skip!();
        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");

        for name in CA05_POINT_SETS {
            let region = regions
                .iter()
                .find(|r| r.name == name)
                .unwrap_or_else(|| panic!("point set {name} must be loaded — a missing set means its enter_region trigger can never fire"));
            assert_eq!(
                region.world_name, "Castle",
                "point set {name} (set_id {}) must belong to World 8 (Castle), got {}",
                region.set_id, region.world_name
            );
            assert_eq!(
                region.points.len(),
                4,
                "point set {name} (set_id {}) must have exactly 4 corner points, has {}",
                region.set_id,
                region.points.len()
            );

            let bx = region_box(&regions, name);
            assert!(
                bx.height() >= MIN_BOX_HEIGHT,
                "point set {name} (set_id {}) has a vertical extent of {:.2} units \
                 (floor {}, ceiling {}), under the {MIN_BOX_HEIGHT} minimum. A box whose \
                 corners all share one y is a zero-height volume: it contains only a \
                 player standing exactly on the floor plane, so the enter_region trigger \
                 effectively never fires and nothing logs an error. Raise the fourth \
                 corner, as every other seeded box does",
                region.set_id,
                bx.height(),
                bx.floor,
                bx.ceiling
            );
        }
    }

    /// **CA05 regression guard**: each story actor stands inside the box the seed
    /// claims contains it.
    ///
    /// Nothing at runtime checks this (see the module comment — region entry is
    /// client-reported), so an actor and its trigger volume can drift apart
    /// silently and the only symptom is a mission step that never fires in-client.
    /// That is exactly what an earlier draft of this packet shipped: both comms
    /// actors sat on the Interrogation Block's floor, 11.6 units below and ~190
    /// units away from the `Castle.CommsRoom` box meant to contain them.
    #[tokio::test]
    async fn castle_ca05_actors_stand_inside_their_claimed_point_set() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");

        for (tag, set_name) in ACTORS_INSIDE {
            let record = spawn_by_tag(&records, tag);
            let bx = region_box(&regions, set_name);
            assert!(
                bx.contains_xz(record.x, record.z),
                "{tag} at ({}, {}, {}) is outside {set_name} in the horizontal plane \
                 (x {}..{}, z {}..{}) — the actor and its trigger volume disagree, and \
                 no runtime check will tell you",
                record.x,
                record.y,
                record.z,
                bx.x.0,
                bx.x.1,
                bx.z.0,
                bx.z.1
            );
            assert!(
                bx.contains_y(record.y),
                "{tag} at y = {} is outside {set_name}'s vertical extent (floor {}, \
                 ceiling {}, {FLOOR_TOLERANCE} units of slack below the floor) — right \
                 footprint, wrong floor, which in a multi-storey interior means a \
                 different room entirely",
                record.y,
                bx.floor,
                bx.ceiling
            );
        }
    }

    /// **CA05 regression guard (negative)**: Warden Muelbach is *outside* Checkpoint
    /// Bravo, and above it.
    ///
    /// Objective 2799 (`mission_objectives.sql`, "(Option #2) Warden Muelbach will
    /// certainly have one. She is holed up in the bunker above Checkpoint Bravo.")
    /// is the whole reason this guard exists. She and the NID Officers are the two
    /// alternatives on mission 708 step 2416, and the content deliberately puts them
    /// in different places; an earlier draft of this packet had her standing in the
    /// Humvee cluster *at* the checkpoint, collapsing the choice into one location.
    /// A revert to that position must fail here — which a containment-only suite of
    /// guards would never catch, since "inside a box" was the wrong thing to want.
    #[tokio::test]
    async fn castle_ca05_muelbach_is_in_the_bunker_above_checkpoint_bravo() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");
        let regions = load_regions_from_db(&pool)
            .await
            .expect("load_regions_from_db must succeed");

        let muelbach = spawn_by_tag(&records, "Castle_Muelbach");
        let bravo = region_box(&regions, "Castle.CheckpointBravo");

        let inside = bravo.contains_xz(muelbach.x, muelbach.z) && bravo.contains_y(muelbach.y);
        assert!(
            !inside,
            "Castle_Muelbach at ({}, {}, {}) is inside Castle.CheckpointBravo \
             (x {}..{}, z {}..{}, y {}..{}), but objective 2799 places her in the \
             bunker ABOVE the checkpoint. Step 2416's two options must not share a \
             location",
            muelbach.x,
            muelbach.y,
            muelbach.z,
            bravo.x.0,
            bravo.x.1,
            bravo.z.0,
            bravo.z.1,
            bravo.floor,
            bravo.ceiling
        );

        // "above" is the load-bearing word in 2799: the bunker is the only Castle
        // bunker asset elevated over the checkpoint, and the two ground-level
        // `EM-Bunker_Frost00` instances were rejected for exactly this reason.
        const MIN_RISE: f32 = 10.0;
        assert!(
            muelbach.y - bravo.floor >= MIN_RISE,
            "Castle_Muelbach at y = {} is only {:.2} units above Castle.CheckpointBravo's \
             floor ({}); objective 2799 says the bunker is ABOVE the checkpoint, and the \
             seeded bunker interior sits {:.0}+ units up. A ground-level bunker was \
             considered and rejected",
            muelbach.y,
            muelbach.y - bravo.floor,
            bravo.floor,
            MIN_RISE
        );
    }

    /// **CA05 regression guard** (worknotes/ca05.md "Zone-wide hostile respawn
    /// timers"): every hostile (faction 10) World 8 spawn row has a non-NULL resolved
    /// `respawn_secs`.
    ///
    /// No shipped Castle row set one, on either `spawnlist` or `entity_templates`, so
    /// the first player to kill any Castle mob removed it from the shared world
    /// permanently — including Romney and the Officers, whose deaths gate missions 703
    /// and 708. Covers the pre-existing rows as well as CA05's own, because the fix
    /// was zone-wide.
    #[tokio::test]
    async fn castle_hostile_world8_spawns_all_have_a_respawn_timer() {
        let pool = require_db_or_skip!();
        let records = load_spawns_from_db(&pool)
            .await
            .expect("load_spawns_from_db must succeed");

        let hostile_castle_spawns: Vec<&SpawnRecord> = records
            .iter()
            .filter(|r| r.world_name == "Castle" && r.faction == Some(10))
            .collect();
        assert!(
            !hostile_castle_spawns.is_empty(),
            "expected at least one hostile (faction 10) World 8 spawn row \
             (NID Guard / Prisoner Retrieval Unit templates 145/146/148, plus \
             Castle_Romney/Castle_Muelbach/Castle_BravoOfficer*) — none found, \
             the probe query itself may be broken"
        );
        for r in &hostile_castle_spawns {
            assert!(
                r.respawn_secs.is_some(),
                "hostile World 8 spawn {} (tag={:?}, template_id={}) has no \
                 resolved respawn_secs — it would permanently disappear from the \
                 shared world after the first kill",
                r.spawn_id,
                r.tag,
                r.template_id
            );
        }
    }
}
