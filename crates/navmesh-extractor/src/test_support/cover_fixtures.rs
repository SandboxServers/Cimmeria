//! Cover-node actors, in the two shapes Castle's chunks cook them in
//! (`docs/reverse-engineering/findings/cover-world-placement.md`).
//!
//! Every cover property is optional on purpose: the cooker omits a
//! property equal to its archetype, and the extractor's class-default
//! handling is only testable if a fixture can leave one out.

use super::chunk_fixtures::{ChunkFixture, Placement, ACTOR_PROPS_OFFSET, COMPONENT_PROPS_OFFSET};

/// The `SGWCoverNodeComponent` cover properties.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CoverMarker {
    pub height: Option<u8>,
    pub quality: Option<u8>,
    pub width: Option<f32>,
}

impl CoverMarker {
    pub fn new(height: u8, quality: u8, width: f32) -> Self {
        Self {
            height: Some(height),
            quality: Some(quality),
            width: Some(width),
        }
    }
}

/// One `CoverNodeArray` child: its own transform plus the absolute flags.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverComponentSpec {
    pub translation: [f32; 3],
    /// UE3 rotator units, `[pitch, yaw, roll]`.
    pub rotation: [i32; 3],
    pub scale_3d: [f32; 3],
    /// Written as all three `Absolute*` bools; `false` omits them, which
    /// is what a relative component looks like.
    pub absolute: bool,
    pub marker: CoverMarker,
}

fn write_marker(props: &mut super::PropStream<'_>, m: &CoverMarker) {
    if let Some(h) = m.height {
        props.byte("CoverHeight", h);
    }
    if let Some(q) = m.quality {
        props.byte("CoverQuality", q);
    }
    if let Some(w) = m.width {
        props.float("CoverWidth", w);
    }
}

impl ChunkFixture {
    /// Pattern A: an `SGWSpecCoverNode` actor and its component. Returns
    /// `(actor, component)` export indices (1-based).
    pub fn add_spec_cover_node(&mut self, at: Placement, marker: CoverMarker) -> (i32, i32) {
        let level = self.level();
        let pkg = self.package_mut();
        let actor_class = pkg.class_ref("SGWSpecCoverNode");
        // Every marker gets the same literal name, as in the cooked chunks —
        // the extractor must not key on it.
        let actor = pkg.add_export(actor_class, level, "SGWSpecCoverNode");
        let comp_class = pkg.class_ref("SGWCoverNodeComponent");
        let comp = pkg.add_export(comp_class, actor, "SGWCoverNodeComponent");

        let mut body = vec![0u8; ACTOR_PROPS_OFFSET];
        let mut props = pkg.props();
        props.object("CoverNodeComponent", comp).placement(
            at.location,
            at.rotation,
            at.draw_scale,
            at.draw_scale_3d,
        );
        body.extend_from_slice(&props.finish());
        pkg.set_payload(actor, body);

        let mut body = vec![0u8; COMPONENT_PROPS_OFFSET];
        let mut props = pkg.props();
        write_marker(&mut props, &marker);
        body.extend_from_slice(&props.finish());
        pkg.set_payload(comp, body);
        (actor, comp)
    }

    /// Pattern B: a `StaticMeshActor` at `owner` whose `CoverNodeArray`
    /// lists one component per spec. Returns `(actor, components)`.
    pub fn add_cover_node_array(
        &mut self,
        owner: Placement,
        children: &[CoverComponentSpec],
    ) -> (i32, Vec<i32>) {
        let level = self.level();
        let pkg = self.package_mut();
        let actor_class = pkg.class_ref("StaticMeshActor");
        let actor = pkg.add_export(actor_class, level, "StaticMeshActor");
        let comp_class = pkg.class_ref("SGWCoverNodeComponent");
        let comps: Vec<i32> = children
            .iter()
            .map(|_| pkg.add_export(comp_class, actor, "SGWCoverNodeComponent"))
            .collect();

        let mut body = vec![0u8; ACTOR_PROPS_OFFSET];
        let mut props = pkg.props();
        props.object_array("CoverNodeArray", &comps).placement(
            owner.location,
            owner.rotation,
            owner.draw_scale,
            owner.draw_scale_3d,
        );
        body.extend_from_slice(&props.finish());
        pkg.set_payload(actor, body);

        for (spec, &comp) in children.iter().zip(&comps) {
            let mut body = vec![0u8; COMPONENT_PROPS_OFFSET];
            let mut props = pkg.props();
            write_marker(&mut props, &spec.marker);
            props
                .vector("Translation", spec.translation)
                .rotator("Rotation", spec.rotation)
                .vector("Scale3D", spec.scale_3d);
            if spec.absolute {
                props
                    .boolean("AbsoluteTranslation", true)
                    .boolean("AbsoluteRotation", true)
                    .boolean("AbsoluteScale", true);
            }
            body.extend_from_slice(&props.finish());
            pkg.set_payload(comp, body);
        }
        (actor, comps)
    }
}
