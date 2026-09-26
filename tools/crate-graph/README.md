# Crate dependency graph

`crate_graph.py` draws the workspace's crate dependency graph as Mermaid from
`cargo metadata` and writes it between the `<!-- crate-graph:begin -->` and
`<!-- crate-graph:end -->` markers in [README.md](../../README.md) and
[crates/README.md](../../crates/README.md).

```bash
python tools/crate-graph/crate_graph.py           # regenerate both READMEs
python tools/crate-graph/crate_graph.py --check   # what CI runs: exit 1 if stale
python tools/crate-graph/crate_graph.py --print   # print the block
python tools/crate-graph/crate_graph.py --full    # every edge, no transitive reduction
```

- **Nodes** are workspace members.
- **Edges** are normal and build dependencies between members. Dev-dependencies are left out, and a crate used only as a dev-dependency is labelled `(dev-only)`.
- **Transitive reduction:** an edge implied by a longer path is omitted, so the picture shows the layering rather than every direct dependency. The caption records how many edges exist and how many are drawn.
- **Groups:** layer groups come from [groups.toml](groups.toml). A crate that isn't listed falls back to a name-prefix rule (`cimmeria-cell-*`, `cimmeria-base-*`) and then to "Other". When you add a crate that belongs to an existing layer, add it there.

Run the script after any change to a `Cargo.toml` dependency list or the workspace `members`. The CI build job fails if you forget. It needs Python 3.11+ (for `tomllib`) and cargo.

The planned target graph for the `cimmeria-services` split is drawn by hand in
[docs/architecture/services-crate-split.md](../../docs/architecture/services-crate-split.md#1-target-crate-map).
