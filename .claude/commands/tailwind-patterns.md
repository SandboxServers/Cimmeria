# Tailwind CSS Patterns (v4)

> Apply these guidelines when writing or reviewing Tailwind CSS in this project. We use Tailwind v4 with the CSS-native configuration approach.

---

## CSS-First Configuration (v4)

v4 uses `@theme` directive instead of `tailwind.config.js`:

```css
@theme {
  --color-primary: oklch(0.7 0.15 250);
  --spacing-md: 1rem;
  --font-sans: 'Inter', system-ui, sans-serif;
}
```

- No JavaScript config file needed
- Oxide engine replaces PostCSS plugin
- Theme values defined as CSS custom properties

## Container Queries

Use container query prefixes for component-responsive layouts:

```html
<div class="@container">
  <div class="@sm:flex @md:grid @lg:grid-cols-3">...</div>
</div>
```

Components respond to parent container width, not viewport.

## Layout & Responsive Design

- Mobile-first: use `sm:`, `md:`, `lg:`, `xl:`, `2xl:` breakpoint prefixes
- Prefer flexbox for stacking/spacing
- Use grid with `auto-fit` for responsive layouts over symmetric grids

## Color & Typography

- Use OKLCH for perceptually uniform colors
- Establish semantic color layers: primitive, semantic, component-specific tokens
- Use font stacks like Space Grotesk (display) + IBM Plex Mono (code) per our project

## Project-Specific Patterns

This project uses:
- `class-variance-authority` (CVA) for component variant management
- `clsx` + `tailwind-merge` for conditional class composition
- `@tailwindcss/vite` plugin (not PostCSS)

Utility helper in `frontend/src/lib/utils.ts`:
```typescript
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

## Best Practices

- Extract components when class combinations repeat 3+ times
- Avoid arbitrary values (`[px]` syntax) — use theme scale
- Avoid `!important` — restructure specificity instead
- Minimize `@apply` usage — prefer utility classes directly
- Use design system scales rather than ad-hoc values
- Group related utilities: layout → spacing → typography → color → effects

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Arbitrary values everywhere | Define theme tokens |
| Heavy `@apply` usage | Use utility classes directly |
| `!important` overrides | Fix specificity properly |
| Inline styles for theming | Use CSS custom properties |
| Duplicate class strings | Extract components or use CVA |
