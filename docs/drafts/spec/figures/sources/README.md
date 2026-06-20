# Figure source DSLs

Source DSL for every diagram embedded in the Mercury wire-format and
entity-property-sync chapters. The rendered SVGs in `../` are the build
artifacts; **these files are the canonical source** and must be edited here
before re-rendering.

These were originally authored in [Prixmaviz](https://prixmaviz.ailuxis.com) and
extracted to disk after a Prixmaviz session became unreachable. Authoring can
continue in Prixmaviz; the files here are kept in sync as a recovery artifact and
to enable offline rendering.

## File-type guide

| Extension | Renderer | Tool / CLI | Diagrams |
|---|---|---|---|
| `.dot` | Graphviz | `dot -Tsvg input.dot -o out.svg` | mercury-01, -02, -22, -23, -24, -36, -37, -38, -39, -40, -41 |
| `.mmd` | Mermaid | `npx -p @mermaid-js/mermaid-cli mmdc -i input.mmd -o out.svg` | mercury-03, -04, -10, -11, -12, -13, -15, -16, -19, -14b, -14c, -27, -32, -33; entity-property-sync-01, -03, -04, -05, -07, -08, -09 |
| `.edn` | bytefield-svg | `bytefield-svg input.edn > out.svg` ([install](https://github.com/Deep-Symmetry/bytefield-svg)) | mercury-05, -06, -08, -14a, -25, -26, -28, -29, -31, -34, -35; entity-property-sync-06 |
| `.json` | WaveDrom | `wavedrom-cli -i input.json -s out.svg` ([install](https://github.com/wavedrom/wavedrom-cli)) | mercury-07; entity-property-sync-02 |
| `.txt` | svgbob | `svgbob input.txt -o out.svg` ([install](https://github.com/ivanceras/svgbob)) | mercury-20, -30 |

## After re-rendering

Every rendered SVG must carry the theme-aware backdrop so it stays readable in
dark-mode markdown viewers. Re-run the post-processor described in commit
`18889d0` (PR #284) after writing any new SVG — it injects `prefers-color-scheme`
CSS, strips graphviz's intrinsic white-fill polygon, and overrides svgbob's
`backdrop` class.

The injector is idempotent via the `<!-- cimmeria-bg-injected -->` marker, so
it's safe to run across the whole `../` directory.

## Why these files exist

The Prixmaviz remote is the convenient authoring surface but it is not a source
of truth for documentation: the diagrams were temporarily inaccessible during
PR #284's last review pass, which would have blocked all repair work if the
source DSLs were not stored here. The Diátaxis principle is that reference
material owns its own source; these files honor that.
