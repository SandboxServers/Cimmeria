# JavaScript Testing Patterns

> Comprehensive testing strategies using Vitest and Testing Library. Apply when writing or reviewing tests in this project.

---

## When to Use

- Setting up test infrastructure
- Writing unit tests for functions, hooks, and classes
- Creating integration tests for API interactions
- Testing React components
- Mocking external dependencies
- Implementing TDD workflows

## Project Test Stack

- **Vitest** — test runner (Vite-native, fast)
- **jsdom** — browser environment simulation
- **React Testing Library** — component testing (behavior-focused)

## Vitest Configuration

```typescript
// vitest.config.ts
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'jsdom',
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      exclude: ['**/*.d.ts', '**/*.config.ts', '**/dist/**'],
    },
    setupFiles: ['./src/test/setup.ts'],
  },
});
```

## Unit Testing Patterns

### Pure Functions

```typescript
import { describe, it, expect } from 'vitest';
import { calculateDamage } from './combat';

describe('calculateDamage', () => {
  it('should apply armor reduction', () => {
    expect(calculateDamage(100, 50)).toBe(50);
  });

  it('should never return negative damage', () => {
    expect(calculateDamage(10, 100)).toBe(0);
  });
});
```

### Async Functions

```typescript
it('should fetch player data', async () => {
  const player = await fetchPlayer('123');
  expect(player).toBeDefined();
  expect(player.name).toBe('TestPlayer');
});

it('should throw on invalid player', async () => {
  await expect(fetchPlayer('invalid')).rejects.toThrow('Not found');
});
```

## Mocking Patterns

### Module Mocking

```typescript
import { vi } from 'vitest';

vi.mock('../lib/admin-api', () => ({
  fetchAdminStatus: vi.fn().mockResolvedValue({
    auth_server: { status: 'online' },
    base_app: { status: 'online' },
  }),
}));
```

### Dependency Injection

```typescript
// Prefer DI over module mocks when possible
interface PlayerRepository {
  findById(id: string): Promise<Player | null>;
}

class PlayerService {
  constructor(private repo: PlayerRepository) {}
  async getPlayer(id: string) { return this.repo.findById(id); }
}

// In tests:
const mockRepo = { findById: vi.fn() };
const service = new PlayerService(mockRepo);
```

### Spying

```typescript
const logSpy = vi.spyOn(console, 'warn');
doSomething();
expect(logSpy).toHaveBeenCalledWith('Expected warning');
logSpy.mockRestore();
```

## React Component Testing

```tsx
import { render, screen, fireEvent } from '@testing-library/react';

describe('PlayerCard', () => {
  it('should display player name', () => {
    render(<PlayerCard player={{ name: 'Teal\'c', level: 50 }} />);
    expect(screen.getByText("Teal'c")).toBeInTheDocument();
  });

  it('should call onSelect when clicked', () => {
    const onSelect = vi.fn();
    render(<PlayerCard player={mockPlayer} onSelect={onSelect} />);
    fireEvent.click(screen.getByRole('button'));
    expect(onSelect).toHaveBeenCalledWith(mockPlayer.id);
  });
});
```

### Hook Testing

```typescript
import { renderHook, act } from '@testing-library/react';

describe('useChainEditor', () => {
  it('should initialize with empty graph', () => {
    const { result } = renderHook(() => useChainEditor());
    expect(result.current.nodes).toHaveLength(0);
  });

  it('should add node', () => {
    const { result } = renderHook(() => useChainEditor());
    act(() => result.current.addNode('trigger'));
    expect(result.current.nodes).toHaveLength(1);
  });
});
```

## Test Organization

```typescript
describe('ChainValidation', () => {
  describe('validateGraph', () => {
    it('should detect empty chains', () => {});
    it('should detect missing sequence anchors', () => {});
    it('should detect unreachable cards', () => {});
  });

  describe('validateNode', () => {
    it('should require title', () => {});
    it('should validate port labels', () => {});
  });
});
```

## Testing Timers

```typescript
it('should debounce autosave', () => {
  vi.useFakeTimers();
  const save = vi.fn();
  triggerAutosave(save);

  expect(save).not.toHaveBeenCalled();
  vi.advanceTimersByTime(5000);
  expect(save).toHaveBeenCalledTimes(1);

  vi.useRealTimers();
});
```

## Best Practices

1. **AAA Pattern** — Arrange, Act, Assert
2. **Test behavior, not implementation** — what the user sees/does
3. **One concept per test** — or logically related assertions
4. **Descriptive names** — `should reject invalid email` not `test1`
5. **Use factories/fixtures** — consistent test data
6. **Clean up** — `afterEach`, `mockRestore()`, prevent test pollution
7. **Prefer semantic queries** — `getByRole`, `getByLabelText` over `getByTestId`
8. **Test edge cases** — empty states, errors, loading, boundaries
9. **Keep tests fast** — mock slow operations, avoid unnecessary waits
10. **Aim for 80%+ coverage** — focus on critical paths

## Project Test Commands

```bash
cd frontend && npm test        # Run all tests
cd frontend && npm run test -- --watch   # Watch mode
cd frontend && npm run test -- --coverage  # Coverage report
```

Current suite: **44 tests passing** across chain editor utilities and admin API.
