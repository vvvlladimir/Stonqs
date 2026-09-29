/**
 * Every screen opens, at every width the layout changes at, without anything going wrong on the
 * way: no uncaught error, no `console.error` (the ErrorBoundary logs there), no command the host
 * answered with an error, and no page wider than the window. Serious accessibility violations
 * fail too — see `KNOWN`.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

/** `ScreenId` minus `plugin`, which needs an installed plugin to be anything. */
const SCREENS = [
  "dashboard",
  "positions",
  "transactions",
  "accounts",
  "securities",
  "watchlist",
  "plans",
  "alerts",
  "performance",
  "trades",
  "risk",
  "allocation",
  "rebalance",
  "income",
  "import",
  "reports",
  "settings",
] as const;

/** Phone, the two-column breakpoint (700px) and a desktop window. */
const WIDTHS = [380, 700, 1280];

/** Everything that went wrong while the page was open, as one readable line each. */
function watch(page: Page): string[] {
  const problems: string[] = [];
  page.on("pageerror", (error) => problems.push(`uncaught: ${error.message}`));
  page.on("console", async (message) => {
    if (message.type() !== "error") return;
    // React's warnings are a format string plus arguments; the component stack is the last one.
    const args = await Promise.all(message.args().map((a) => a.jsonValue().catch(() => "?")));
    const [format, ...rest] = args.map(String);
    let i = 0;
    const text = (format ?? message.text()).replace(/%s/g, () => rest[i++] ?? "");
    problems.push(`console.error: ${text.split("\n").slice(0, 4).join(" ")}`);
  });
  page.on("response", async (response) => {
    const url = new URL(response.url());
    if (!url.pathname.startsWith("/__host/")) return;
    const command = url.pathname.slice("/__host/".length);
    if (response.status() === 404) {
      problems.push(`${command}: the browser host has no route for it (examples/browser_host/routes.rs)`);
      return;
    }
    const body = (await response.json().catch(() => null)) as { err?: unknown; desktop_only?: true } | null;
    // A command the browser host cannot run (it needs the desktop window) is not a bug.
    if (body && "err" in body && !body.desktop_only)
      problems.push(`${command} answered ${JSON.stringify(body.err)}`);
  });
  return problems;
}

const isHost = (url: string) => new URL(url).pathname.startsWith("/__host/");

/**
 * Resolves once no host request has been in flight for `quietMs`: a chart asks after its data, so
 * one empty moment is not the end. Counted rather than `networkidle`, which waits a fixed 500ms.
 */
function hostTraffic(page: Page) {
  let inflight = 0;
  let last = Date.now();
  page.on("request", (r) => {
    if (isHost(r.url())) (inflight++, (last = Date.now()));
  });
  const done = (r: { url(): string }) => {
    if (isHost(r.url())) (inflight--, (last = Date.now()));
  };
  page.on("requestfinished", done);
  page.on("requestfailed", done);
  return async (quietMs: number) => {
    while (inflight > 0 || Date.now() - last < quietMs) await page.waitForTimeout(50);
  };
}

async function horizontalOverflow(page: Page): Promise<number> {
  return page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
}

async function seriousViolations(page: Page): Promise<string[]> {
  // WCAG A and AA only: the best-practice rules are advice, and they double the time on a big table.
  const { violations } = await new AxeBuilder({ page })
    .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"])
    .analyze();
  return violations
    .filter((v) => v.impact === "serious" || v.impact === "critical")
    .flatMap((v) => v.nodes.map((n) => ({ rule: v.id, target: n.target.join(" "), why: n.failureSummary })))
    .filter((found) => !KNOWN.some((k) => k.rule === found.rule && found.target.includes(k.where)))
    .map((found) => `${found.rule}: ${found.target} — ${found.why?.split("\n")[1]?.trim()}`);
}

// One page load per screen: the widths and the accessibility check all read the same page.
for (const screen of SCREENS) {
  test(screen, async ({ page }, info) => {
    const problems = watch(page);
    const settle = hostTraffic(page);
    await page.setViewportSize({ width: WIDTHS[WIDTHS.length - 1], height: 900 });
    await page.goto(`/?screen=${screen}`);
    await expect(page.locator("main")).toBeVisible();
    await settle(400);

    // The slowest check by far, and the same rules in every engine: asked of one project only.
    if (info.project.name === "webkit")
      expect.soft(await seriousViolations(page), "serious accessibility violations").toEqual([]);

    for (const width of [...WIDTHS].reverse()) {
      await page.setViewportSize({ width, height: 900 });
      await settle(150);
      expect
        .soft(await horizontalOverflow(page), `the page scrolls sideways at ${width}px`)
        .toBeLessThanOrEqual(0);
    }

    expect(problems, "problems while the screen was open").toEqual([]);
  });
}

/**
 * Violations known today, each a debt to pay rather than a rule switched off: a rule and the
 * element it fails on, so the same rule anywhere else still fails. Remove an entry once it passes.
 */
const KNOWN: Array<{ rule: string; where: string }> = [
  // A scrolling box with nothing focusable in it cannot be scrolled from the keyboard.
  { rule: "scrollable-region-focusable", where: ".w__body" },
  { rule: "scrollable-region-focusable", where: ".tree" },
  // The income calendar's grid role wants rows of cells it does not declare.
  { rule: "aria-required-children", where: ".cal" },
  // A month without data in a calendar is a button whose only name is its tooltip.
  { rule: "button-name", where: "button[data-tip=" },
  // Text on a coloured surface: tabs, chips, heatmap cells, chart labels in SVG.
  { rule: "color-contrast", where: ".tabs__tab" },
  { rule: "color-contrast", where: ".chip" },
  { rule: "color-contrast", where: ".cal__v" },
  { rule: "color-contrast", where: "text[" },
];
