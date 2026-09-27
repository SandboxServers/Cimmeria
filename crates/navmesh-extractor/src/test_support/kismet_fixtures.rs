//! Kismet and Matinee objects on a [`ChunkFixture`] (NA40): the chain
//! `interp_actor::MotionEvidence` follows from a `SeqAct_Interp` to the
//! actors its groups move.
//!
//! ```text
//! SeqAct_Interp.VariableLinks
//!   [Data]        -> InterpData.InterpGroups -> InterpGroup(GroupName)
//!                                                 .InterpTracks -> InterpTrackMove
//!   [<GroupName>] -> SeqVar_Object.ObjValue -> the actor
//! ```
//!
//! Kismet and Matinee objects carry a 4-byte prefix before their
//! properties.

use super::chunk_fixtures::ChunkFixture;

/// Bytes before a Kismet/Matinee object's property stream.
pub const KISMET_PROPS_OFFSET: usize = 4;

/// One `InterpTrackMove`'s keys.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveKeys {
    /// `true` writes `MoveFrame = IMF_RelativeToInitial`; `false` leaves
    /// it out, which the reader takes as `IMF_World`.
    pub relative: bool,
    /// `PosTrack` key values, UE3 cm.
    pub positions: Vec<[f32; 3]>,
    /// `EulerTrack` key values, degrees.
    pub rotations: Vec<[f32; 3]>,
}

impl MoveKeys {
    /// Relative-frame track through `positions`, no rotation.
    pub fn relative(positions: &[[f32; 3]]) -> Self {
        Self {
            relative: true,
            positions: positions.to_vec(),
            rotations: vec![[0.0; 3]; positions.len()],
        }
    }

    /// Relative-frame track that stays put and turns through
    /// `rotations`.
    pub fn turning(rotations: &[[f32; 3]]) -> Self {
        Self {
            relative: true,
            positions: vec![[0.0; 3]; rotations.len()],
            rotations: rotations.to_vec(),
        }
    }
}

/// One group of a Matinee: its name, the `SeqVar_Object`s linked to
/// it, and its tracks.
#[derive(Debug, Clone)]
pub struct GroupSpec<'a> {
    pub name: &'a str,
    pub seqvars: Vec<i32>,
    pub moves: Vec<MoveKeys>,
    /// Class names of extra, keyless tracks (`InterpTrackEvent`, ...).
    pub other_tracks: Vec<&'a str>,
}

impl ChunkFixture {
    /// A Kismet export with `body` after the 4-byte prefix.
    fn add_kismet_export(&mut self, class: &str, name: &str, body: Vec<u8>) -> i32 {
        let level = self.level();
        let pkg = self.package_mut();
        let class_ref = pkg.class_ref(class);
        let export = pkg.add_export(class_ref, level, name);
        let mut payload = vec![0u8; KISMET_PROPS_OFFSET];
        payload.extend_from_slice(&body);
        pkg.set_payload(export, payload);
        export
    }

    /// A `SeqVar_Object` holding `object`.
    pub fn add_seqvar_object(&mut self, object: i32) -> i32 {
        let mut props = self.package_mut().props();
        props.object("ObjValue", object);
        let body = props.finish();
        self.add_kismet_export("SeqVar_Object", "SeqVar_Object", body)
    }

    /// A Kismet op of `class` with one variable link `desc` to
    /// `seqvars` — e.g. a `SeqAct_Toggle` targeting an actor.
    pub fn add_kismet_op(&mut self, class: &str, desc: &str, seqvars: &[i32]) -> i32 {
        let link = self.variable_link(desc, seqvars);
        let mut props = self.package_mut().props();
        props.struct_array("VariableLinks", &[link]);
        let body = props.finish();
        self.add_kismet_export(class, class, body)
    }

    /// A Kismet export with one object property `prop` naming `object`
    /// directly — e.g. a `SeqEvent_Touch.Originator`.
    pub fn add_kismet_object_ref(&mut self, class: &str, prop: &str, object: i32) -> i32 {
        let mut props = self.package_mut().props();
        props.object(prop, object);
        let body = props.finish();
        self.add_kismet_export(class, class, body)
    }

    /// A whole Matinee: tracks, groups, `InterpData`, and the
    /// `SeqAct_Interp` wiring it to the groups' `SeqVar_Object`s.
    /// Returns the `SeqAct_Interp` export.
    pub fn add_matinee(&mut self, groups: &[GroupSpec<'_>]) -> i32 {
        let mut group_exports = Vec::new();
        for g in groups {
            let mut tracks = Vec::new();
            for m in &g.moves {
                let body = self.move_track_body(m);
                tracks.push(self.add_kismet_export("InterpTrackMove", "InterpTrackMove", body));
            }
            for class in &g.other_tracks {
                let body = self.package_mut().props().finish();
                tracks.push(self.add_kismet_export(class, class, body));
            }
            let mut props = self.package_mut().props();
            props
                .name_value("GroupName", g.name)
                .object_array("InterpTracks", &tracks);
            let body = props.finish();
            group_exports.push(self.add_kismet_export("InterpGroup", "InterpGroup", body));
        }
        let mut props = self.package_mut().props();
        props.object_array("InterpGroups", &group_exports);
        let body = props.finish();
        let data = self.add_kismet_export("InterpData", "InterpData", body);

        let mut links = vec![self.variable_link("Data", &[data])];
        for g in groups {
            links.push(self.variable_link(g.name, &g.seqvars));
        }
        let mut props = self.package_mut().props();
        props.struct_array("VariableLinks", &links);
        let body = props.finish();
        self.add_kismet_export("SeqAct_Interp", "SeqAct_Interp", body)
    }

    /// One `SeqVarLink` struct element.
    fn variable_link(&mut self, desc: &str, vars: &[i32]) -> Vec<u8> {
        let mut props = self.package_mut().props();
        props
            .string("LinkDesc", desc)
            .object_array("LinkedVariables", vars);
        props.finish()
    }

    fn move_track_body(&mut self, m: &MoveKeys) -> Vec<u8> {
        let pos = self.curve(&m.positions);
        let euler = self.curve(&m.rotations);
        let mut props = self.package_mut().props();
        props
            .struct_value("PosTrack", "InterpCurveVector", &pos)
            .struct_value("EulerTrack", "InterpCurveVector", &euler);
        if m.relative {
            props.byte("MoveFrame", 1);
        }
        props.finish()
    }

    /// An `InterpCurveVector` body: `Points`, one struct per key, keys
    /// one second apart.
    fn curve(&mut self, values: &[[f32; 3]]) -> Vec<u8> {
        let points: Vec<Vec<u8>> = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let mut p = self.package_mut().props();
                p.float("InVal", i as f32).vector("OutVal", *v);
                p.finish()
            })
            .collect();
        let mut props = self.package_mut().props();
        props.struct_array("Points", &points);
        props.finish()
    }
}
