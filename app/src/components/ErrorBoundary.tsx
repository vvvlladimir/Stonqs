import { Trans } from "@lingui/react/macro";
import { Component, type ErrorInfo, type ReactNode } from "react";
import { ErrorText } from "./ui";

/**
 * Shows the exception instead of a blank window: React unmounts the whole tree on
 * a render error, and a white rectangle gives no clue where to look.
 */
interface State {
  error: Error | null;
}

export class ErrorBoundary extends Component<{ children: ReactNode }, State> {
  state: State = { error: null };

  static getDerivedStateFromError(error: Error): State {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    // Keep the stack visible in the development UI; the Tauri console is manual.
    console.error("render failure", error, info.componentStack);
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <div className="app">
        <ErrorText as="div">
          <strong>
            <Trans>Interface failure</Trans>
          </strong>
          <p>{this.state.error.message}</p>
          <pre className="mono">{this.state.error.stack}</pre>
        </ErrorText>
        <button className="btn" onClick={() => this.setState({ error: null })}>
          <Trans>Try again</Trans>
        </button>
      </div>
    );
  }
}
