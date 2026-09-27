// The whole of what a plugin page knows about the app (ADR-0083/0084): one message in, carrying
// the context and the reads the manifest declared, and three out — ready, an error, and a screen's
// own document to keep.
(() => {
  "use strict";
  // The policy governs every request a page can make except a peer connection, whose STUN server
  // is the page's to choose — enough to carry data out. Gone before the module runs, and a page
  // that may open no frame has no fresh copy to fetch it back from.
  for (const name of Object.getOwnPropertyNames(window)) {
    if (!/^(webkit|moz)?RTC/.test(name)) continue;
    try {
      delete window[name];
    } catch {
      // Not configurable: fall through to overwriting it.
    }
    if (window[name] !== undefined) {
      try {
        Object.defineProperty(window, name, { value: undefined });
      } catch {
        // Neither deletable nor redefinable: nothing further this page can do about it.
      }
    }
  }

  const host = window.parent;
  let renderer = null;
  let last = null;

  const post = (message) => host.postMessage({ stonqs: 1, ...message }, "*");
  const fail = (code, error) =>
    post({ type: "error", code, detail: String((error && error.message) || error || "").slice(0, 500) });

  const apply = (context) => {
    const root = document.documentElement;
    root.lang = context.locale || "";
    root.style.colorScheme = context.theme.scheme;
    for (const [name, value] of Object.entries(context.theme.tokens)) root.style.setProperty(name, value);
  };

  const run = () => {
    if (!renderer || !last) return;
    try {
      const out = renderer(document.body, last);
      if (out && typeof out.then === "function") out.then(null, (e) => fail("threw", e));
    } catch (e) {
      fail("threw", e);
    }
  };

  window.stonqs = Object.freeze({
    api: 1,
    /** `fn(root, { context, data })`, called again whenever either changes. */
    render(fn) {
      if (typeof fn !== "function") throw new TypeError("stonqs.render takes a function");
      renderer = fn;
      run();
    },
    /** Replaces the plugin's one document in the profile; it comes back as `data.state` on the
     *  next render. Only a screen declaring `storage` is answered (ADR-0084). */
    save(state) {
      post({ type: "save", state: JSON.parse(JSON.stringify(state)) });
    },
  });

  window.addEventListener("message", (event) => {
    if (event.source !== host) return;
    const message = event.data;
    if (!message || message.stonqs !== 1 || message.type !== "render") return;
    last = { context: message.context, data: message.data };
    apply(message.context);
    run();
  });
  window.addEventListener("error", (e) => fail("threw", e.error || e.message));
  window.addEventListener("unhandledrejection", (e) => fail("threw", e.reason));
  // A module runs before `load`, top-level await aside, so a renderer still missing here is one
  // that will never come.
  window.addEventListener("load", () => {
    if (!renderer) fail("no_render", "");
  });

  post({ type: "ready" });
})();
