//! Полифиллы для тестов.
//! matchMedia не реализован в jsdom, а CentsGauge читает prefers-reduced-motion.
//! Channel из @tauri-apps/api требует window.__TAURI_INTERNALS__ — в тестах
//! сообщения всё равно доставляются напрямую через onmessage.

if (typeof window !== 'undefined') {
  if (!window.matchMedia) {
    const stub = (query: string): MediaQueryList =>
      ({
        matches: false,
        media: query,
        onchange: null,
        addListener: () => {},
        removeListener: () => {},
        addEventListener: () => {},
        removeEventListener: () => {},
        dispatchEvent: () => false,
      }) as unknown as MediaQueryList;
    window.matchMedia = stub;
  }

  const w = window as unknown as Record<string, unknown>;
  w.__TAURI_INTERNALS__ ??= {
    transformCallback: () => Math.floor(Math.random() * 2 ** 32),
  };
}
