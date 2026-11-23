//! Полифиллы для компонентных тестов в jsdom.
//! matchMedia не реализован в jsdom, а CentsGauge читает prefers-reduced-motion.

if (typeof window !== 'undefined' && !window.matchMedia) {
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
