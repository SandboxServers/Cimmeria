//! `AbilityTreeCatalog`: every archetype's training tree, loaded once from
//! `resources.archetype_ability_tree` joined to `resources.abilities`.

use std::collections::HashMap;

use sqlx::PgPool;

/// One node of an archetype's ability tree: one `resources.archetype_ability_tree`
/// row plus the ability's raw `training_cost`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeNode {
    /// `EArchetype` enum position (0=Any, 1=Soldier, 2=Commando, ...).
    pub archetype_id: i32,
    /// Branch, 0..=2 (`tree_index_sanity`).
    pub tree_index: i32,
    /// Position within the branch (`ability_index`). Catalog order is
    /// `tree_index, node_order`.
    pub node_order: i32,
    pub ability_id: i32,
    /// Minimum player level to train the node.
    pub level: i32,
    /// Abilities the player must already know (`prerequisite_abilities`).
    pub prerequisites: Vec<i32>,
    /// Archetype-wide tree points that must be spent before this node opens.
    pub required_branch_points: i32,
    /// Training points the node costs.
    pub skill_point_cost: i32,
    pub is_branch_root: bool,
    pub is_capstone: bool,
    pub branch_name: Option<String>,
    /// `resources.abilities.training_cost`, as authored. Informational: the
    /// debit uses `skill_point_cost`. Never rewritten.
    pub raw_training_cost: i32,
}

impl TreeNode {
    /// A node carrying the schema defaults for every column the stub seed
    /// does not set (`required_branch_points = 0`, `skill_point_cost = 1`,
    /// not a root, not a capstone, no branch name, raw cost 0).
    /// `node_order` is 0; the catalog keeps insertion order.
    pub fn with_defaults(
        archetype_id: i32,
        tree_index: i32,
        ability_id: i32,
        level: i32,
        prerequisites: Vec<i32>,
    ) -> Self {
        Self {
            archetype_id,
            tree_index,
            node_order: 0,
            ability_id,
            level,
            prerequisites,
            required_branch_points: 0,
            skill_point_cost: 1,
            is_branch_root: false,
            is_capstone: false,
            branch_name: None,
            raw_training_cost: 0,
        }
    }
}

/// Every archetype's tree, in catalog order, with an `(archetype, ability)`
/// index for the per-node lookups the trainer and the purchase gate make.
#[derive(Debug, Clone, Default)]
pub struct AbilityTreeCatalog {
    by_archetype: HashMap<i32, Vec<TreeNode>>,
    /// `(archetype_id, ability_id)` → position in `by_archetype[archetype_id]`.
    index: HashMap<(i32, i32), usize>,
}

impl AbilityTreeCatalog {
    /// Build a catalog from nodes already in catalog order.
    pub fn from_nodes(nodes: impl IntoIterator<Item = TreeNode>) -> Self {
        let mut catalog = Self::default();
        for node in nodes {
            catalog.push(node);
        }
        catalog
    }

    /// Append a node to its archetype's tree. A second node with the same
    /// `(archetype, ability)` is kept in the tree but never returned by
    /// [`Self::node`]; the schema's `UNIQUE (archetype, ability_id)` makes
    /// that unreachable from the database.
    pub fn push(&mut self, node: TreeNode) {
        let tree = self.by_archetype.entry(node.archetype_id).or_default();
        self.index
            .entry((node.archetype_id, node.ability_id))
            .or_insert(tree.len());
        tree.push(node);
    }

    /// The archetype's tree in catalog order; empty when it has none.
    pub fn tree(&self, archetype_id: i32) -> &[TreeNode] {
        self.by_archetype
            .get(&archetype_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// The node for `ability_id` in the archetype's tree.
    pub fn node(&self, archetype_id: i32, ability_id: i32) -> Option<&TreeNode> {
        let &pos = self.index.get(&(archetype_id, ability_id))?;
        self.by_archetype.get(&archetype_id)?.get(pos)
    }

    /// Number of archetypes with at least one node.
    pub fn archetype_count(&self) -> usize {
        self.by_archetype.len()
    }

    /// Total nodes across every archetype.
    pub fn len(&self) -> usize {
        self.by_archetype.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.by_archetype.is_empty()
    }

    /// Load every archetype's tree.
    ///
    /// The archetype id is the `EArchetype` enum position, the same encoding
    /// `sgw_player.archetype` and the cell entity use. Rows arrive ordered
    /// `tree_index, ability_index` within each archetype, which is the order
    /// the trainer and `onAbilityTreeInfo` present. The inner join is safe:
    /// `archetype_ability_tree_ability_id_fkey` guarantees every row has an
    /// ability.
    pub async fn load(pool: &PgPool) -> Result<Self, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct Row {
            archetype_id: i32,
            tree_index: i32,
            ability_index: i32,
            ability_id: i32,
            level: i32,
            prerequisite_abilities: Vec<i32>,
            required_branch_points: i32,
            skill_point_cost: i32,
            is_branch_root: bool,
            is_capstone: bool,
            branch_name: Option<String>,
            raw_training_cost: i32,
        }

        let rows = sqlx::query_as::<_, Row>(
            "SELECT \
                 array_position(enum_range(NULL::resources.\"EArchetype\"), t.archetype) - 1 \
                     AS archetype_id, \
                 t.tree_index, t.ability_index, t.ability_id, t.level, \
                 t.prerequisite_abilities, t.required_branch_points, \
                 t.skill_point_cost, t.is_branch_root, t.is_capstone, \
                 t.branch_name, a.training_cost AS raw_training_cost \
             FROM resources.archetype_ability_tree t \
             JOIN resources.abilities a ON a.ability_id = t.ability_id \
             ORDER BY t.archetype, t.tree_index, t.ability_index",
        )
        .fetch_all(pool)
        .await?;

        let catalog = Self::from_nodes(rows.into_iter().map(|r| TreeNode {
            archetype_id: r.archetype_id,
            tree_index: r.tree_index,
            node_order: r.ability_index,
            ability_id: r.ability_id,
            level: r.level,
            prerequisites: r.prerequisite_abilities,
            required_branch_points: r.required_branch_points,
            skill_point_cost: r.skill_point_cost,
            is_branch_root: r.is_branch_root,
            is_capstone: r.is_capstone,
            branch_name: r.branch_name,
            raw_training_cost: r.raw_training_cost,
        }));

        tracing::info!(
            archetypes = catalog.archetype_count(),
            total_entries = catalog.len(),
            "Loaded archetype ability trees"
        );
        Ok(catalog)
    }
}
