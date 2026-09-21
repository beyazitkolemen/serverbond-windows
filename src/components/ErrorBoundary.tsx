import { Component, type ErrorInfo, type ReactNode } from "react";
import { APP_NAME } from "../product";

type State = { failed: boolean; detail: string; resetKey: unknown };

type Props = {
  children: ReactNode;
  /**
   * Scoped mode: render an inline notice instead of replacing the whole
   * window, and recover automatically when this value changes (for example
   * when the user navigates to another page).
   */
  resetKey?: unknown;
  scope?: string;
};

export default class ErrorBoundary extends Component<Props, State> {
  state: State = { failed: false, detail: "", resetKey: undefined };

  static getDerivedStateFromError(error: unknown): Partial<State> {
    return {
      failed: true,
      detail: error instanceof Error ? error.message : String(error),
    };
  }

  static getDerivedStateFromProps(
    props: Props,
    state: State,
  ): Partial<State> | null {
    if (state.failed && props.resetKey !== state.resetKey) {
      return { failed: false, detail: "", resetKey: props.resetKey };
    }
    if (props.resetKey !== state.resetKey) {
      return { resetKey: props.resetKey };
    }
    return null;
  }

  componentDidCatch(error: unknown, info: ErrorInfo) {
    // The Rust side keeps running; keep the failure discoverable in the
    // WebView console for support instead of dropping it silently.
    console.error(
      `${APP_NAME} arayüz hatası${this.props.scope ? ` (${this.props.scope})` : ""}`,
      error,
      info.componentStack,
    );
  }

  render() {
    if (!this.state.failed) return this.props.children;
    if (this.props.scope) {
      return (
        <section className="pane-error" role="alert">
          <h2>{this.props.scope} görüntülenemedi.</h2>
          <p>
            Diğer sayfalar ve çalışan hizmetler etkilenmez. Başka bir sayfaya
            geçip geri dönün ya da yeniden deneyin.
          </p>
          {this.state.detail ? (
            <pre className="error-detail">{this.state.detail}</pre>
          ) : null}
          <button
            type="button"
            className="button secondary"
            onClick={() => this.setState({ failed: false, detail: "" })}
          >
            Yeniden dene
          </button>
        </section>
      );
    }
    return (
      <main className="loading-state" role="alert">
        <h1>Arayüz görüntülenemedi.</h1>
        <p>
          Çalışan servisler devam edebilir. Arayüzü yeniden yükleyip durumlarını
          kontrol edin.
        </p>
        {this.state.detail ? (
          <pre className="error-detail">{this.state.detail}</pre>
        ) : null}
        <button
          className="button primary"
          onClick={() => window.location.reload()}
        >
          Arayüzü yeniden yükle
        </button>
        <p>
          Hata sürerse tepsi menüsündeki Çıkış komutuyla {APP_NAME}&apos;ı
          kapatıp yeniden açın.
        </p>
      </main>
    );
  }
}
