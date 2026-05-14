# Figure source DSLs

Source DSL for every diagram embedded in the Mercury wire-format chapter. The
rendered SVGs in `../` are the build artifacts; **these files are the canonical
source** and must be edited here before re-rendering.

These were originally authored in [Prixmaviz](https://prixmaviz.ailuxis.com) and
extracted to disk after a Prixmaviz session became unreachable. Authoring can
continue in Prixmaviz; the files here are kept in sync as a recovery artifact and
to enable offline rendering.

## File-type guide

| Extension | Renderer | Tool / CLI | Diagrams |
|---|---|---|---|
| `.dot` | Graphviz | `dot -Tsvg input.dot -o out.svg` | mercury-01, -02, -22, -23, -24 |
| `.mmd` | Mermaid | `npx -p @mermaid-js/mermaid-cli mmdc -i input.mmd -o out.svg` | mercury-03, -04, -10, -11, -12, -13, -15, -16, -19 |
| `.edn` | bytefield-svg | `bytefield-svg input.edn > out.svg` ([install](https://github.com/Deep-Symmetry/bytefield-svg)) | mercury-05, -06, -08, -25 |
| `.json` | WaveDrom | `wavedrom-cli -i input.json -s out.svg` ([install](https://github.com/wavedrom/wavedrom-cli)) | mercury-07 |
| `.txt` | svgbob | `svgbob input.txt -o out.svg` ([install](https://github.com/ivanceras/svgbob)) | mercury-14, -20 |

## After re-rendering

Every rendered SVG must carry the theme-aware backdrop so it stays readable in
dark-mode markdown viewers. Re-run the post-processor described in commit
`18889d0` (PR #284) after writing any new SVG — it injects `prefers-color-scheme`
CSS, strips graphviz's intrinsic white-fill polygon, and overrides svgbob's
`backdrop` class.

The injector is idempotent via the `<!-- cimmeria-bg-injected -->` marker, so
it's safe to run across the whole `../` directory.

## Known issues to fix on next authoring pass

- **mercury-07-flags-register** — bit assignments for bits 5/6/7 do not match the
  chapter's canonical §1.2 table. Bit 5 should be `FLAG_HAS_SEQUENCE_NUMBER`,
  bit 6 `FLAG_HAS_REQUESTS`, bit 7 `FLAG_IS_FRAGMENT`. `INDEXED` (`0x80`) does
  not exist in SGW and should be removed entirely.
- **mercury-25-ack-list-tail-encoding** — `ackCount` 1-cell span is too narrow
  for `ackCount : u8` label, which overlaps the adjacent "end of datagram"
  cell. Widen `ackCount` to 2 cells and reduce one of the neighbors.
- **mercury-12-encryption-pipeline** — subgraph titles wrap onto two lines and
  collide with the cluster border. Shorten to single-line, e.g.
  `"ENCRYPT — MercuryEncryption::encrypt"`. Add Mermaid init directive
  `%%{init: {"flowchart": {"htmlLabels": false}}}%%` so labels render as
  native SVG text (foreignObject is dropped by markdown viewers loading SVG
  via `<img>`).
- **mercury-13-codec-encode-decode** — same `htmlLabels: false` fix.
- **mercury-15-message-dispatch-routing** — same `htmlLabels: false` fix
  (decision diamonds render empty without it).
- **mercury-19-resource-fragment-paths** — same `htmlLabels: false` fix
  (diamond `\n` line breaks collapse to spaces without it).

## Why these files exist

The Prixmaviz remote is the convenient authoring surface but it is not a source
of truth for documentation: the diagrams were temporarily inaccessible during
PR #284's last review pass, which would have blocked all repair work if the
source DSLs were not stored here. The Diátaxis principle is that reference
material owns its own source; these files honor that.
