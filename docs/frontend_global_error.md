# Frontend Global Error Boundary for Documentation

## Overview
The `FrontendGlobalErrorBoundary` is a React class component designed to catch runtime JavaScript errors within the Documentation section of the `stellar-raise-contracts` frontend application. By intercepting these errors, it prevents them from crashing the entire React component tree, ensuring that the rest of the application remains functional and displaying a user-friendly fallback UI.

## Usage
Wrap the generic components or specifically the documentation wrappers with the error boundary component.

```tsx
import { FrontendGlobalErrorBoundary } from '../components/frontend_global_error';

function DocumentationPage() {
  return (
    <FrontendGlobalErrorBoundary>
      <DocumentationContent />
    </FrontendGlobalErrorBoundary>
  );
}
```

### Custom Fallback Customization
If the default fallback UI is insufficient, you can provide a custom `fallback` node via props.

```tsx
<FrontendGlobalErrorBoundary fallback={<div>Custom documentation crash message.</div>}>
  <DocumentationContent />
</FrontendGlobalErrorBoundary>
```

## Security & Reliability Assumptions
- **XSS Prevention**: By catching unexpected UI errors, the boundary prevents any corrupted rendering states that might inadvertently expose raw sensitive debug information to the DOM. The fallback UI strictly relies on standard string interpolation preventing XSS.
- **Enhanced Isolation**: Documentation code heavily relies on rendering external or dense markdown content. The error boundary acts as a blast door isolating the core platform (like investment flows or onboarding) from documentation failures.
- **Fail-safe mechanism**: The boundary provides a built-in "Try Again" recovery mechanism. It clears the internal error state, prompting a re-render of the child component tree without needing a full page reload, improving UX.

## Testing and Coverage
The component is fully unit-tested using Jest and `@testing-library/react`. Tests encompass:
1. Standard non-error render validation.
2. Caught error validation with the default fallback and simulated console error logs.
3. Custom fallback render validation.
4. Error recovery ("Try Again" logic) flow validation ensuring state is properly updated. Minimum 95% test coverage is enforced.
