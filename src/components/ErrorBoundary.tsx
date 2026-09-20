import { Component, type ErrorInfo, type ReactNode } from "react";
import { APP_NAME } from "../product";

type State = { failed: boolean; detail: string };

export default class ErrorBoundary extends Component<
  { children: ReactNode },
  State
> {
  state: State = { failed: false, detail: "" };

  static getDerivedStateFromError(error: unknown): State {
    return {
      failed: true,
      detail: error instanceof Error ? error.message : String(error),
    };
  }

  componentDidCatch(error: unknown, info: ErrorInfo) {
    // The Rust side keeps running; keep the failure discoverable in the
    // WebView console for support instead of dropping it silently.
    console.error(`${APP_NAME} arayüz hatası`, error, info.componentStack);
  }

  render() {
    if (this.state.failed) {
      return (
        <main className="loading-state" role="alert">
          <h1>Arayüz görüntülenemedi.</h1>
          <p>
            Çalışan servisler devam edebilir. Arayüzü yeniden yükleyip
            durumlarını kontrol edin.
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
    return this.props.children;
  }
}
