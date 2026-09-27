// The whole of what a widget page knows about the app (ADR-0083): one message in, carrying the
// context and the reads the manifest declared, and three out — ready, an error, nothing else.
(() => {
  "use strict";
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
