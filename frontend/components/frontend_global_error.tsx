import React, { Component, ErrorInfo, ReactNode } from 'react';

/**
 * @title Frontend Global Error Boundary for Documentation
 * @dev NatSpec: This React class component serves as a global error boundary
 * tailored specifically for the Documentation section of the frontend app.
 * It catches runtime JavaScript errors anywhere in its child component tree,
 * logs those errors, and displays a fallback UI instead of crashing the component tree.
 * 
 * @custom:security By catching errors and rendering a static safe fallback, 
 * this boundary prevents potential side effects of broken UI state and limits
 * exposure of raw error stack traces to end users (mitigating information disclosure).
 * 
 * @custom:efficiency This boundary operates purely on the client side, introducing 
 * negligible overhead during normal render cycles. It only incurs cost when an 
 * exception is thrown and caught.
 */
interface Props {
  /**
   * @dev The child components to render inside the boundary.
   */
  children?: ReactNode;
  
  /**
   * @dev Optional custom fallback UI to render when an error is caught.
   * If not provided, a default stylized documentation error message is shown.
   */
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class FrontendGlobalErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  /**
   * @dev Updates state so the next render will show the fallback UI.
   * @param error The error that was thrown.
   * @return The new state object indicating an error has occurred.
   */
  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  /**
   * @dev Lifecycle method invoked after an error has been thrown by a descendant component.
   * Used for logging error information.
   * @param error The error that was thrown.
   * @param errorInfo An object with a `componentStack` key containing information about which component threw the error.
   */
  componentDidCatch(error: Error, errorInfo: ErrorInfo): void {
    // In a production app, we would log to an external service (e.g. Sentry/Datadog)
    console.error("Documentation Error Boundary caught an error:", error, errorInfo);
  }

  render(): ReactNode {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div 
          className="documentation-error-boundary" 
          style={{ padding: '20px', border: '1px solid #ff4d4f', borderRadius: '4px', backgroundColor: '#fff2f0', color: '#cf1322' }}
          role="alert"
        >
          <h2>Documentation Loading Error</h2>
          <p>We're sorry, but the documentation content failed to load due to an unexpected error.</p>
          <details style={{ whiteSpace: 'pre-wrap', marginTop: '10px' }}>
            <summary>Error Details</summary>
            {this.state.error?.message}
          </details>
          <button 
            onClick={() => this.setState({ hasError: false, error: null })}
            style={{ marginTop: '10px', padding: '8px 16px', cursor: 'pointer', backgroundColor: '#cf1322', color: '#fff', border: 'none', borderRadius: '4px' }}
          >
            Try Again
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
