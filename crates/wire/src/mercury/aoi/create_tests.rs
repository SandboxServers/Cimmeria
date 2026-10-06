//! Tests for the NPC createOnClient cascade in [`super`]: its id base, the appearance steps and the name steps.

#[cfg(test)]
mod cascade_idbase_tests {
    use super::super::*;

    /// NPC cascade — `npc_data` present → NPC idbase.
    #[test]
    fn cascade_idbase_npc_branch_returns_npc_default() {
        let npc = NpcAoIData::default();
        assert_eq!(cascade_idbase(Some(&npc)), IDBASE_NPC_DEFAULT);
    }

    /// Player-ghost cascade — `npc_data` absent → SGWPlayer idbase.
    #[test]
    fn cascade_idbase_player_branch_returns_sgw_player() {
        assert_eq!(cascade_idbase(None), IDBASE_SGW_PLAYER);
    }
}

#[cfg(test)]
mod appearance_cascade_tests {
    use super::super::*;

    /// A cascade whose NPC has NO appearance data (no static_mesh, no
    /// body_set/components) emits the `aoi.cascade_appearance_missing`
    /// negative-log — the seam that surfaces an entity which will be
    /// invisible to witnesses. Reverting the `warn!` in `append_appearance`
    /// trips this. (`#[tokio::test]` because `LogCapture::install` requires
    /// the current-thread runtime.)
    #[tokio::test]
    async fn no_appearance_data_emits_warn() {
        let capture = crate::test_support::LogCapture::install();

        let npc = NpcAoIData {
            static_mesh: None,
            body_set: None,
            components: vec![],
            ..NpcAoIData::default()
        };
        let _ = compose_create_entity_cascade_body(4242, 0x00, 1, Some(&npc));

        let event = capture
            .find_event(
                tracing::Level::WARN,
                "no appearance data",
                "no_appearance_data",
            )
            .expect("missing appearance must emit the aoi.cascade_appearance_missing warn");
        assert!(
            event.has_field("entity_id", "4242"),
            "warn must carry the entity_id field: {event:#?}"
        );
    }

    /// Companion guard: an NPC WITH a static mesh (the real Castle_CellBlock
    /// corpse shape — body_set present but components empty, so the
    /// static-mesh branch fires) must NOT trip the appearance-missing warn.
    /// Proves the seam doesn't false-positive on well-formed props.
    #[tokio::test]
    async fn static_mesh_present_does_not_warn() {
        let capture = crate::test_support::LogCapture::install();

        let npc = NpcAoIData {
            static_mesh: Some("CA-Props.CA-GuardCorpse02".to_string()),
            body_set: Some("GLB_Components.WorldObject_Small".to_string()),
            components: vec![],
            ..NpcAoIData::default()
        };
        let _ = compose_create_entity_cascade_body(4243, 0x00, 1, Some(&npc));

        assert!(
            capture
                .find_event(
                    tracing::Level::WARN,
                    "no appearance data",
                    "no_appearance_data",
                )
                .is_none(),
            "a static-mesh NPC must not trip the appearance-missing warn"
        );
    }
}

#[cfg(test)]
mod being_name_id_tests {
    use super::super::*;

    fn with_name_id() -> NpcAoIData {
        NpcAoIData {
            static_mesh: Some("CA-Props.CA-GuardCorpse02".to_string()),
            body_set: Some("GLB_Components.WorldObject_Small".to_string()),
            name_id: Some(4711),
            ..NpcAoIData::default()
        }
    }

    /// The `onBeingNameIDUpdate` message: direct msg id `0x80 + 11`, the
    /// entity id, then the INT32 name id (NPC idbase 62 > 11, so direct).
    fn name_id_message(entity_id: u32) -> Vec<u8> {
        // 0x8B = 0x80 | 11; u16 LE payload length 8 (entity id + INT32).
        let mut m = vec![0x8B, 0x08, 0x00];
        m.extend_from_slice(&entity_id.to_le_bytes());
        m.extend_from_slice(&4711i32.to_le_bytes());
        m
    }

    fn contains(hay: &[u8], needle: &[u8]) -> bool {
        hay.windows(needle.len()).any(|w| w == needle)
    }

    /// A class-0 prop with a name id gets no `onBeingNameIDUpdate`: the
    /// client has no handler for it on `SGWSpawnableEntity`. Reverting the
    /// class gate puts the message back and fails this.
    #[test]
    fn a_spawnable_entity_prop_gets_no_being_name_id() {
        let body = compose_create_entity_cascade_body(100179, 0x00, 1, Some(&with_name_id()));
        assert!(!contains(&body, &name_id_message(100179)));
    }

    /// An SGWMob with the same data still gets it, byte for byte.
    #[test]
    fn a_mob_still_gets_its_being_name_id() {
        let msg = name_id_message(100010);
        let body = compose_create_entity_cascade_body(
            100010,
            crate::mercury::SGWMOB_CLASS_ID,
            1,
            Some(&with_name_id()),
        );
        assert!(
            contains(&body, &msg),
            "the mob cascade carries onBeingNameIDUpdate"
        );
    }

    /// The `onBeingNameUpdate` message for `name`: direct msg id `0x80 +
    /// 17`, a u16 payload length, the entity id, then the WSTRING (u32 code
    /// unit count, UTF-16LE).
    fn being_name_message(entity_id: u32, name: &str) -> Vec<u8> {
        let units: Vec<u16> = name.encode_utf16().collect();
        let len = 4 + 4 + units.len() * 2;
        let mut m = vec![0x91];
        m.extend_from_slice(&(len as u16).to_le_bytes());
        m.extend_from_slice(&entity_id.to_le_bytes());
        m.extend_from_slice(&(units.len() as u32).to_le_bytes());
        for u in units {
            m.extend_from_slice(&u.to_le_bytes());
        }
        m
    }

    /// A template with a `display_name` (the Visual NPC Lineup) sends it as
    /// `onBeingNameUpdate`, byte for byte, and no `onBeingNameIDUpdate`: the
    /// client's nameplate draws a mob's name-id text over any literal name
    /// (lab, 2026-10-05). Revert proof: drop step 5b, or send the name id
    /// again, and this fails.
    #[test]
    fn a_display_name_replaces_the_name_id_as_being_name_update() {
        let name = "Teal'c #30 BS_JaffaMale";
        let npc = NpcAoIData {
            display_name: Some(name.to_string()),
            ..with_name_id()
        };
        let body = compose_create_entity_cascade_body(
            100011,
            crate::mercury::SGWMOB_CLASS_ID,
            1,
            Some(&npc),
        );
        let find = |needle: &[u8]| body.windows(needle.len()).position(|w| w == needle);
        assert!(
            find(&being_name_message(100011, name)).is_some(),
            "onBeingNameUpdate carries the literal name"
        );
        assert!(
            find(&name_id_message(100011)).is_none(),
            "no name id, which the nameplate would draw instead"
        );
    }

    /// No `display_name` (every shipped template), or an empty one, sends no
    /// `onBeingNameUpdate`; a prop never gets one either.
    #[test]
    fn no_display_name_sends_no_being_name_update() {
        let header = |id: u32| {
            let mut h = vec![0x91];
            h.extend_from_slice(&[0, 0]);
            h.extend_from_slice(&id.to_le_bytes());
            h
        };
        let has_name_update = |body: &[u8], id: u32| {
            let h = header(id);
            body.windows(h.len())
                .any(|w| w[0] == h[0] && w[3..] == h[3..])
        };
        for name in [None, Some(String::new())] {
            let npc = NpcAoIData {
                display_name: name,
                ..with_name_id()
            };
            let body = compose_create_entity_cascade_body(
                100012,
                crate::mercury::SGWMOB_CLASS_ID,
                1,
                Some(&npc),
            );
            assert!(!has_name_update(&body, 100012));
            let id = name_id_message(100012);
            assert!(
                body.windows(id.len()).any(|w| w == id.as_slice()),
                "the name id is still sent"
            );
        }
        let prop = NpcAoIData {
            display_name: Some("Crate".into()),
            ..with_name_id()
        };
        let body = compose_create_entity_cascade_body(100013, 0x00, 1, Some(&prop));
        assert!(!has_name_update(&body, 100013));
    }

    #[test]
    fn only_sgwbeing_descendants_bind_being_methods() {
        for id in 0x01..=0x05 {
            assert!(class_binds_being_methods(id), "class {id}");
        }
        for id in [0x00, 0x06, 0x07] {
            assert!(!class_binds_being_methods(id), "class {id}");
        }
    }
}
