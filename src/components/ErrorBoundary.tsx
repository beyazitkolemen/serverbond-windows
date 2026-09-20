import { Component, type ReactNode } from "react";

export default class ErrorBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
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
          <button
            className="button primary"
            onClick={() => window.location.reload()}
          >
            Arayüzü yeniden yükle
          </button>
          <p>
            Hata sürerse tepsi menüsündeki Çıkış komutuyla F4Box'ı kapatıp
            yeniden açın.
          </p>
        </main>
      );
    }
    return this.props.children;
  }
}
