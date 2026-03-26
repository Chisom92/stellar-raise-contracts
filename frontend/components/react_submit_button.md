# React Submit Button — Dependencies

Documents the runtime dependencies, peer requirements, dev toolchain, and upgrade guidance for `react_submit_button.tsx`.

---

## Runtime Dependencies

The component has **zero production dependencies beyond React itself**.

| Package | Version range | Role | Justification |
|---------|--------------|------|---------------|
| `react` | `^19.0.0` | Peer — JSX, hooks (`useState`) | Required for component rendering |
| `react-dom` | `^19.0.0` | Peer — DOM rendering | Required by the consuming app; not imported directly |

No third-party UI libraries, animation libraries, or utility packages are imported. This keeps the bundle contribution of this component to a minimum and eliminates transitive dependency risk.

---

## Dev / Test Dependencies

| Package | Version range | Role |
|---------|--------------|------|
| `@testing-library/react` | `^16.0.0` | `render`, `screen`, `fireEvent`, `act` |
| `@testing-library/jest-dom` | `^6.0.0` | Extended matchers (used via `jest.setup.ts`) |
| `@testing-library/user-event` | `^14.0.0` | Available for future interaction tests |
| `jest` | `^30.0.0` | Test runner |
| `jest-environment-jsdom` | `^30.0.0` | DOM environment for React tests |
| `ts-jest` | `^29.0.0` | TypeScript transform for Jest |
| `typescript` | `^5.0.0` | Type checking |
| `@types/react` | `^19.0.0` | React type definitions |
| `@types/jest` | `^30.0.0` | Jest type definitions |

All dev dependencies are declared in the workspace `package.json` and are not shipped to production.

---

## Peer Dependency Requirements

### React 19

The component uses:

- `useState` — local in-flight submission guard
- `React.MouseEvent<HTMLButtonElement>` — typed click handler
- `React.CSSProperties` — inline style typing
- `react-jsx` transform — no explicit `React` import needed at call sites

React 18 is also compatible. The only React 19 feature used is the `react-jsx` automatic JSX transform, which was introduced in React 17. **Minimum supported React version: 17**.

### TypeScript 4.7+

The component uses:

- Const type parameters (TypeScript 5.0) — not used; compatible with TS 4.7+
- Template literal types — not used
- Strict union types and `Record<K, V>` — available since TS 4.1

**Minimum supported TypeScript version: 4.7**.

---

## What the Component Does NOT Depend On

| Excluded dependency | Reason |
|--------------------|--------|
| `classnames` / `clsx` | Class logic is a single optional prop passthrough |
| `styled-components` / `emotion` | Styles are inline `React.CSSProperties` constants |
| `framer-motion` / `react-spring` | Transitions use CSS `transition` property only |
| `react-hook-form` | Component is form-library agnostic |
| `zustand` / `redux` | State is fully controlled by the parent via the `state` prop |
| `lodash` | No utility functions needed |
| `uuid` | No ID generation needed |

Keeping the dependency surface minimal reduces:
- Bundle size impact on the consuming app
- Supply-chain attack surface
- Version conflict risk in monorepos

---

## Dependency Graph

```
react_submit_button.tsx
└── react          (peer, runtime)
    └── react-dom  (peer, runtime — consuming app only)

react_submit_button.test.tsx
├── react                        (peer)
├── @testing-library/react       (dev)
├── @testing-library/jest-dom    (dev, via jest.setup.ts)
└── react_submit_button.tsx      (local)
```

---

## Version Pinning Policy

The workspace `package.json` uses caret ranges (`^`) for all dependencies. This allows non-breaking minor and patch updates while preventing automatic major version upgrades.

For production deployments, pin exact versions in `package-lock.json` (already committed) and run `npm ci` rather than `npm install` to guarantee reproducible installs.

---

## Upgrading React

### React 17 → 18

No changes required. The component does not use `ReactDOM.render` (removed in React 18) or any deprecated lifecycle methods.

### React 18 → 19

No changes required. The component does not use:
- `ReactDOM.render` (removed in 18)
- `act` from `react-dom/test-utils` (moved to `react` in 19 — tests already import from `@testing-library/react`)
- Concurrent features that changed API between 18 and 19

### Upgrading `@testing-library/react`

The test suite uses `render`, `screen`, `fireEvent`, and `act` — all stable APIs present since `@testing-library/react` v13. No breaking changes are expected through v16.

---

## Security Assumptions Related to Dependencies

1. **No `dangerouslySetInnerHTML`** — the component never uses this API, so XSS via label content is not possible regardless of React version.
2. **No dynamic `import()`** — the component is statically bundled; no code-splitting attack surface.
3. **No network calls** — the component is purely presentational; it emits events to the parent and renders state. No fetch, axios, or WebSocket dependency.
4. **Inline styles only** — no CSS-in-JS runtime that could be exploited via style injection. All colour values are hardcoded constants in `STATE_STYLES`.

---

## NatSpec-style Reference

### Dependency contract
- **@notice** `react_submit_button.tsx` has one runtime peer dependency: `react ≥ 17`.
- **@dev** All other imports are local types and constants — no third-party runtime packages.
- **@security** Zero third-party runtime dependencies eliminates transitive supply-chain risk for this component.

---

## Running Tests

```bash
# Run with coverage report
npx jest --testPathPatterns=react_submit_button --coverage

# Watch mode during development
npm run test:watch -- --testPathPatterns=react_submit_button
```

### Latest test output

```
Tests:    51 passed, 51 total
Coverage: 97.67% statements | 96.87% branches | 100% functions | 100% lines
```
