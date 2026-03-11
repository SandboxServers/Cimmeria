# Frontend Design

> Create distinctive, production-grade frontend interfaces with intentional aesthetics, high craft, and non-generic visual identity. Apply when building or reviewing UI in this project.

---

## Core Mandate

Every output must satisfy four requirements:

1. **Intentional aesthetic direction** — no defaults
2. **Technical correctness** — semantic, accessible HTML
3. **Visual memorability** — recognizable without branding
4. **Cohesive restraint** — unified, not cluttered

Reject: default layouts, design-by-components, "safe" palettes.

## Design Feasibility & Impact Index (DFII)

Score designs from -5 to +15 across five dimensions:

| Dimension | Weight |
|-----------|--------|
| Aesthetic impact | High |
| Context fit | High |
| Implementation feasibility | Medium |
| Performance safety | Medium |
| Consistency risk | Low |

- Scores 12-15: Full execution
- Scores 4-11: Refine approach
- Scores ≤3: Rethink entirely

## Design Thinking Phase

Before coding, define:

1. **Purpose** — what is this interface for?
2. **Tone** — one dominant direction (e.g., brutalism, luxury, retro-futurism, technical, editorial)
3. **Differentiation anchor** — what makes this recognizable without branding?

## Aesthetic Execution Rules

### Typography
- One expressive display font + one restrained body font
- Avoid overused defaults (Inter, Roboto, Arial) unless justified
- This project uses: **Space Grotesk** (display) + **IBM Plex Mono** (code/data)

### Color
- One dominant tone, one accent, one neutral system
- Define via CSS custom properties
- Use OKLCH for perceptual uniformity

### Composition
- Intentional asymmetry and overlap where appropriate
- Strategic negative space
- Grid-breaking elements for visual interest

### Motion
- Purposeful and sparse — every animation should communicate something
- CSS-first animations (prefer transitions and keyframes)

### Texture
- Use when narratively justified (e.g., sci-fi admin panel aesthetic)
- Avoid gratuitous gradients or shadows

## Required Output Structure

When creating new UI:

1. Design direction summary (2-3 sentences)
2. Design system snapshot (colors, fonts, spacing)
3. Full working code
4. Differentiation callout (what makes this distinctive)

## Anti-Patterns (Immediate Failure)

- Generic font stacks with no thought
- Default framework layouts (centered card, generic sidebar)
- Overused AI design tropes (glassmorphism for no reason, excessive blur)
- Decoration without communicative intent
- Inconsistent spacing or color usage

## Project Context

This is an **admin panel for a Stargate Worlds MMO server emulator**. The aesthetic should reflect:
- Technical/military command-center feel
- Data-dense interfaces that remain readable
- Sci-fi without being cartoonish
- Professional tool for developers and GMs
