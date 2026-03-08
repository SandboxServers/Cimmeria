# React Patterns

> Apply these patterns when writing React components in this project. We use React 19+ with TypeScript.

---

## 1. Component Design

| Type | Use | State |
|------|-----|-------|
| **Presentational** | UI display | Props only |
| **Container** | Logic/state | Heavy state |
| **Client** | Interactivity | useState, effects |

### Rules

- One responsibility per component
- Props down, events up
- Composition over inheritance
- Prefer small, focused components

## 2. Hook Patterns

### When to Extract Hooks

| Pattern | Extract When |
|---------|-------------|
| **useLocalStorage** | Same storage logic needed |
| **useDebounce** | Multiple debounced values |
| **useFetch** | Repeated fetch patterns |
| **useForm** | Complex form state |

### Rules

- Hooks at top level only
- Same order every render
- Custom hooks start with `use`
- Clean up effects on unmount

## 3. State Management Selection

| Complexity | Solution |
|------------|----------|
| Simple | useState, useReducer |
| Shared local | Context |
| Server state | React Query, SWR |
| Complex global | Zustand, Redux Toolkit |

| Scope | Where |
|-------|-------|
| Single component | useState |
| Parent-child | Lift state up |
| Subtree | Context |
| App-wide | Global store |

## 4. React 19 Patterns

### New Hooks

| Hook | Purpose |
|------|---------|
| **useActionState** | Form submission state |
| **useOptimistic** | Optimistic UI updates |
| **use** | Read resources in render |

### Compiler Benefits

- Automatic memoization (less manual useMemo/useCallback)
- Focus on writing pure components

## 5. Composition Patterns

### Compound Components

- Parent provides context
- Children consume context
- Flexible slot-based composition
- Example: Tabs, Accordion, Dropdown

### Render Props vs Hooks

| Use Case | Prefer |
|----------|--------|
| Reusable logic | Custom hook |
| Render flexibility | Render props |
| Cross-cutting | Higher-order component |

## 6. Performance

| Signal | Action |
|--------|--------|
| Slow renders | Profile first |
| Large lists | Virtualize |
| Expensive calc | useMemo |
| Stable callbacks | useCallback |

**Optimization order:** Check if actually slow → Profile with DevTools → Identify bottleneck → Apply targeted fix.

## 7. Error Handling

- Use Error Boundaries at route/feature level
- Show fallback UI on error
- Log errors and offer retry option
- Preserve user data across errors

## 8. TypeScript Patterns

| Pattern | Use |
|---------|-----|
| Interface | Component props |
| Type | Unions, complex types |
| Generic | Reusable components |

| Need | Type |
|------|------|
| Children | ReactNode |
| Event handler | MouseEventHandler |
| Ref | RefObject\<Element\> |

## 9. Anti-Patterns

| Don't | Do |
|-------|-----|
| Prop drilling deep | Use context |
| Giant components | Split smaller |
| useEffect for everything | Consider alternatives |
| Premature optimization | Profile first |
| Index as key | Stable unique ID |
| Mutate state directly | Return new objects |

## Project-Specific Notes

- This project uses React 19 with TypeScript
- UI components use CVA (class-variance-authority) for variants
- Testing with Vitest + React Testing Library patterns
- ReactFlow/XY Flow for graph editor components — see `/react-flow-architect`
