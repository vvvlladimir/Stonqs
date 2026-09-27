// Position heat map: one cell per position, its area the position's weight, its colour the
// unrealized result over cost. That percentage is this plugin's own figure (ADR-0082) — the app
// shows value and result, not this ratio — so the cell says what it is.

const percent = (locale) =>
  new Intl.NumberFormat(locale || undefined, { style: "percent", maximumFractionDigits: 1, signDisplay: "exceptZero" });

/** How strongly a result colours its cell: full strength at ±25%, never quite invisible. */
const strength = (result) => Math.round(18 + Math.min(Math.abs(result) / 0.25, 1) * 62);

function cell(row, format) {
  const cost = Number(row.cost_basis);
  const result = cost > 0 ? Number(row.unrealized_result) / cost : 0;
  const tone = result >= 0 ? "var(--pos)" : "var(--neg)";
  const el = document.createElement("div");
  el.title = `${row.name}: ${format.format(result)} on cost`;
  el.style.cssText = [
    `flex: ${Number(row.weight) * 1000} 1 ${Math.max(Number(row.weight) * 100, 12)}%`,
    `background: color-mix(in srgb, ${tone} ${strength(result)}%, var(--surface-2))`,
    "min-height: 44px",
    "border-radius: var(--r-sm, 6px)",
    "padding: 6px 8px",
    "box-sizing: border-box",
    "overflow: hidden",
    "display: flex",
    "flex-direction: column",
    "justify-content: space-between",
  ].join(";");

  const symbol = document.createElement("strong");
  symbol.textContent = row.symbol || row.name;
  const figure = document.createElement("span");
  figure.textContent = format.format(result);
  figure.style.cssText = "font-variant-numeric: tabular-nums; opacity: .85";
  el.append(symbol, figure);
  return el;
}

stonqs.render((root, { context, data }) => {
  root.style.cssText = [
    "margin: 0",
    "height: 100%",
    "display: flex",
    "flex-wrap: wrap",
    "align-content: stretch",
    "gap: 3px",
    "color: var(--text)",
    "font: var(--fs-sm, 13px) / 1.3 system-ui, sans-serif",
  ].join(";");

  const rows = (data.positions?.rows ?? [])
    .filter((row) => Number(row.value) > 0)
    .sort((a, b) => Number(b.weight) - Number(a.weight));
  if (rows.length === 0) {
    const empty = document.createElement("p");
    empty.textContent = "No positions on this date.";
    empty.style.cssText = "margin: auto; color: var(--text-3)";
    root.replaceChildren(empty);
    return;
  }
  const format = percent(context.locale);
  root.replaceChildren(...rows.map((row) => cell(row, format)));
});
