/** What jsdom does not bring and a screen expects to be there. Nothing here is a stub of the
 *  app's own code: the host is mocked at the IPC door instead (`src/test/host.tsx`). */
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";

// The stylesheet asks for the colour scheme and jsdom has no media queries at all.
if (!window.matchMedia) {
  window.matchMedia = ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  })) as typeof window.matchMedia;
}

// A tooltip and the row tables measure themselves; jsdom reports zero for everything, which is
// a layout the components already handle — but not the observer being absent.
if (!globalThis.ResizeObserver) {
  globalThis.ResizeObserver = class {
    observe() {}
    unobserve() {}
    disconnect() {}
  } as unknown as typeof ResizeObserver;
}

if (!Element.prototype.scrollIntoView) {
  Element.prototype.scrollIntoView = () => {};
}

afterEach(() => {
  cleanup();
  clearMocks();
});
