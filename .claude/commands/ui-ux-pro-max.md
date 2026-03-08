# UI/UX Design Systems

> Premium design system guidance covering tokens, component architecture, accessibility, and interaction patterns. Apply when building or reviewing component libraries and design tokens.

---

## Design Token Architecture

### Token Layers

```
Primitive Tokens (raw values)
  → Semantic Tokens (purpose-mapped)
    → Component Tokens (scoped to components)
```

### Example Structure

```css
/* Primitive */
--gray-900: oklch(0.15 0 0);
--blue-500: oklch(0.6 0.15 250);

/* Semantic */
--color-bg-primary: var(--gray-900);
--color-text-primary: var(--gray-100);
--color-accent: var(--blue-500);

/* Component */
--button-bg: var(--color-accent);
--card-bg: var(--color-bg-primary);
```

### Spacing Scale

Use a consistent spacing scale (4px base):

```
--space-1: 0.25rem   /* 4px */
--space-2: 0.5rem    /* 8px */
--space-3: 0.75rem   /* 12px */
--space-4: 1rem      /* 16px */
--space-6: 1.5rem    /* 24px */
--space-8: 2rem      /* 32px */
--space-12: 3rem     /* 48px */
```

## Component Architecture

### Variant System (CVA)

This project uses `class-variance-authority` for component variants:

```typescript
import { cva, type VariantProps } from 'class-variance-authority';

const button = cva('inline-flex items-center justify-center rounded-md font-medium', {
  variants: {
    variant: {
      default: 'bg-accent text-accent-foreground hover:bg-accent/90',
      destructive: 'bg-destructive text-destructive-foreground',
      outline: 'border border-input bg-transparent hover:bg-accent',
      ghost: 'hover:bg-accent hover:text-accent-foreground',
    },
    size: {
      sm: 'h-8 px-3 text-xs',
      md: 'h-9 px-4 text-sm',
      lg: 'h-10 px-6 text-base',
    },
  },
  defaultVariants: {
    variant: 'default',
    size: 'md',
  },
});
```

### Component Composition Rules

1. **Props interface** — always typed with TypeScript
2. **Variant support** — use CVA for multi-variant components
3. **className passthrough** — accept and merge external classes via `cn()`
4. **Ref forwarding** — use `forwardRef` for interactive elements
5. **Accessible defaults** — ARIA attributes, keyboard handling built-in

## Accessibility Checklist

### Critical (must have)
- [ ] Color contrast ratio ≥ 4.5:1 for text, ≥ 3:1 for large text
- [ ] All interactive elements keyboard-accessible
- [ ] Focus indicators visible and styled
- [ ] ARIA labels on icon-only buttons
- [ ] Screen reader announcements for dynamic content

### High Priority
- [ ] Touch targets ≥ 44x44px
- [ ] Reduced motion support (`prefers-reduced-motion`)
- [ ] Error messages associated with form fields
- [ ] Logical tab order

### Medium Priority
- [ ] Skip navigation links
- [ ] Heading hierarchy (h1 → h2 → h3)
- [ ] Meaningful link text (not "click here")

## Interaction Patterns

### Data Tables
- Sortable columns with clear indicators
- Pagination or virtual scrolling for large datasets
- Row selection with keyboard support
- Empty state with clear messaging

### Forms
- Inline validation on blur
- Clear error messages near the field
- Loading states on submit buttons
- Disabled state styling

### Modals/Dialogs
- Focus trap inside modal
- Close on Escape key
- Return focus to trigger on close
- Backdrop click to dismiss (optional)

### Navigation
- Active state clearly indicated
- Keyboard navigable
- Collapsible sidebar for admin panels
- Breadcrumbs for deep hierarchies

## Dark Mode / Theme Support

```css
/* Use CSS custom properties for theming */
:root {
  --bg-primary: oklch(0.98 0 0);
  --text-primary: oklch(0.15 0 0);
}

[data-theme="dark"] {
  --bg-primary: oklch(0.12 0 0);
  --text-primary: oklch(0.92 0 0);
}
```

## Pre-Delivery Checklist

- [ ] Visual consistency across all pages
- [ ] Light and dark mode contrast verified
- [ ] All interactive states styled (hover, focus, active, disabled)
- [ ] Loading skeletons for async content
- [ ] Empty states designed
- [ ] Error states designed
- [ ] Responsive at all breakpoints (if applicable)
- [ ] Performance: no layout shift, images optimized

## Project Context

This admin panel uses:
- **Tailwind CSS v4** with CVA for variants
- **Space Grotesk** (headings) + **IBM Plex Mono** (data/code)
- Dark-themed command-center aesthetic
- Data-dense layouts with clear hierarchy
