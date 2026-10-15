import { Component, type ReactNode } from "react";

type PanelBoundaryProps = {
  /** Changing it clears a previous failure, so switching panels recovers. */
  resetKey: string | number;
  /** Shown, inside an alert, when the children fail to render. */
  fallback: ReactNode;
  /** Receives the error; the caller decides how to log it. */
  onError?: (error: unknown) => void;
  children?: ReactNode;
};

type PanelBoundaryState = {
  failed: boolean;
  resetKey: string | number;
};

/**
 * Isolates a panel failure: the panel shows `fallback` and the rest of the
 * app keeps running. Logs nothing by itself, to keep error details out of
 * the console; reporting goes through `onError`.
 */
class PanelBoundary extends Component<PanelBoundaryProps, PanelBoundaryState> {
  state: PanelBoundaryState = {
    failed: false,
    resetKey: this.props.resetKey,
  };

  static getDerivedStateFromProps(
    props: PanelBoundaryProps,
    state: PanelBoundaryState,
  ): Partial<PanelBoundaryState> | null {
    if (Object.is(props.resetKey, state.resetKey)) return null;
    return { failed: false, resetKey: props.resetKey };
  }

  static getDerivedStateFromError(): Partial<PanelBoundaryState> {
    return { failed: true };
  }

  componentDidCatch(error: unknown) {
    this.props.onError?.(error);
  }

  render() {
    if (this.state.failed) {
      return <div role="alert">{this.props.fallback}</div>;
    }
    return this.props.children;
  }
}

export default PanelBoundary;
