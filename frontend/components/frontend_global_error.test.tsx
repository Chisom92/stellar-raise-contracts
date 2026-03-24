import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { FrontendGlobalErrorBoundary } from './frontend_global_error';

// Suppress console.error in tests to cleanly handle expected error boundaries output
const originalConsoleError = console.error;
beforeAll(() => {
  console.error = jest.fn();
});

afterAll(() => {
  console.error = originalConsoleError;
});

const ThrowError = ({ message = "Test error" }) => {
  throw new Error(message);
};

describe('FrontendGlobalErrorBoundary', () => {
  beforeEach(() => {
    jest.clearAllMocks();
  });

  it('renders children correctly when there is no error', () => {
    render(
      <FrontendGlobalErrorBoundary>
        <div data-testid="child">Safe Content</div>
      </FrontendGlobalErrorBoundary>
    );

    expect(screen.getByTestId('child')).toBeTruthy();
    expect(screen.getByText('Safe Content')).toBeTruthy();
  });

  it('renders default fallback UI when an error is thrown', () => {
    render(
      <FrontendGlobalErrorBoundary>
        <ThrowError message="Simulated documentation crash" />
      </FrontendGlobalErrorBoundary>
    );

    expect(screen.getByRole('alert')).toBeTruthy();
    expect(screen.getByText('Documentation Loading Error')).toBeTruthy();
    expect(screen.getByText('Simulated documentation crash')).toBeTruthy();
    
    // Verify componentDidCatch was called via logging
    expect(console.error).toHaveBeenCalledWith(
      "Documentation Error Boundary caught an error:",
      expect.any(Error),
      expect.objectContaining({ componentStack: expect.any(String) })
    );
  });

  it('renders a custom fallback UI when provided', () => {
    const CustomFallback = <div data-testid="custom-fallback">Custom Error View</div>;

    render(
      <FrontendGlobalErrorBoundary fallback={CustomFallback}>
        <ThrowError message="Another crash" />
      </FrontendGlobalErrorBoundary>
    );

    expect(screen.getByTestId('custom-fallback')).toBeTruthy();
    expect(screen.getByText('Custom Error View')).toBeTruthy();
    expect(screen.queryByText('Documentation Loading Error')).toBeNull();
  });

  it('allows user to recovery by clicking "Try Again"', () => {
    const mockChild = jest.fn();
    let shouldThrow = true;

    const RecoverableComponent = () => {
      mockChild();
      if (shouldThrow) {
        throw new Error("Temporary error");
      }
      return <div>Recovered Content</div>;
    };

    const { rerender } = render(
      <FrontendGlobalErrorBoundary>
        <RecoverableComponent />
      </FrontendGlobalErrorBoundary>
    );

    // Initial render throws
    expect(screen.getByText('Documentation Loading Error')).toBeTruthy();

    // User fixes the issue externally (e.g. data loads correctly now)
    shouldThrow = false;

    // Click "Try Again"
    const retryButton = screen.getByRole('button', { name: "Try Again" });
    fireEvent.click(retryButton);

    // Re-renders child successfully
    expect(screen.getByText('Recovered Content')).toBeTruthy();
    expect(screen.queryByText('Documentation Loading Error')).toBeNull();
  });
});
