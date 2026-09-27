// Spending: every withdrawal of the period sorted into a category by the first rule whose text the
// operation's note contains. The rules are this plugin's one document in the profile; the totals
// are its own figures (ADR-0082) — the app itself has no notion of a category of spending.

const UNSORTED = "Unsorted";

const el = (tag, style, ...children) => {
  const node = document.createElement(tag);
  if (style) node.style.cssText = style;
  node.append(...children.filter((c) => c !== null && c !== undefined));
  return node;
};

const categoryOf = (rules, note) => {
  const text = (note || "").toLowerCase();
  const rule = rules.find((r) => r.match && text.includes(r.match.toLowerCase()));
  return rule ? rule.category : UNSORTED;
};

function totals(rows, rules) {
  const by = new Map();
  for (const row of rows) {
    const category = categoryOf(rules, row.note);
    by.set(category, (by.get(category) || 0) + Math.abs(Number(row.amount_base)));
  }
  return [...by].sort((a, b) => b[1] - a[1]);
}

/** The notes no rule answers yet, largest first: what the user is asked about next. */
function unsorted(rows, rules) {
  const by = new Map();
  for (const row of rows) {
    if (categoryOf(rules, row.note) !== UNSORTED) continue;
    const note = (row.note || "").trim();
    by.set(note, (by.get(note) || 0) + Math.abs(Number(row.amount_base)));
  }
  return [...by].sort((a, b) => b[1] - a[1]).slice(0, 12);
}

const button = (label, onClick) => {
  const b = el(
    "button",
    "font: inherit; color: var(--text); background: var(--surface-2); border: 1px solid var(--border); " +
      "border-radius: var(--r-sm, 6px); padding: 4px 10px; cursor: pointer",
    label,
  );
  b.type = "button";
  b.addEventListener("click", onClick);
  return b;
};

const input = (value, placeholder) => {
  const i = el(
    "input",
    "font: inherit; color: var(--text); background: var(--surface); border: 1px solid var(--border); " +
      "border-radius: var(--r-sm, 6px); padding: 4px 8px; min-width: 0; flex: 1",
  );
  i.value = value;
  i.placeholder = placeholder;
  return i;
};

const heading = (text) => el("h2", "font-size: var(--fs-base, 15px); margin: 20px 0 8px", text);

stonqs.render((root, { context, data }) => {
  const rules = Array.isArray(data.state?.rules) ? data.state.rules : [];
  const save = (next) => stonqs.save({ rules: next });
  const money = new Intl.NumberFormat(context.locale || undefined, {
    style: "currency",
    currency: context.base_currency || "EUR",
    maximumFractionDigits: 0,
  });
  const rows = (data.transactions?.rows ?? []).filter((r) => r.kind === "WITHDRAWAL");
  const spent = rows.reduce((sum, r) => sum + Math.abs(Number(r.amount_base)), 0);

  root.style.cssText =
    "margin: 0; padding: 4px 2px 24px; color: var(--text); font: var(--fs-sm, 13px) / 1.4 system-ui, sans-serif";

  if (rows.length === 0) {
    root.replaceChildren(
      el("p", "color: var(--text-3)", "No withdrawals in this period — nothing was spent from the accounts in view."),
    );
    return;
  }

  const bars = totals(rows, rules).map(([category, amount], i) =>
    el(
      "div",
      "display: grid; grid-template-columns: 9em 1fr 7em; gap: 10px; align-items: center; margin: 4px 0",
      el("span", "overflow: hidden; text-overflow: ellipsis; white-space: nowrap", category),
      el(
        "div",
        "height: 10px; border-radius: 5px; background: var(--surface-2)",
        el(
          "div",
          `height: 100%; border-radius: 5px; width: ${(amount / spent) * 100}%; background: ${
            category === UNSORTED ? "var(--text-3)" : `var(--slot-${(i % 8) + 1})`
          }`,
        ),
      ),
      el("span", "text-align: right; font-variant-numeric: tabular-nums", money.format(amount)),
    ),
  );

  const asks = unsorted(rows, rules).map(([note, amount]) => {
    const match = input(note, "text the note contains");
    const category = input("", "category");
    return el(
      "div",
      "display: flex; gap: 8px; align-items: center; margin: 4px 0",
      el("span", "width: 7em; text-align: right; font-variant-numeric: tabular-nums", money.format(amount)),
      match,
      category,
      button("Sort", () => {
        const text = match.value.trim();
        const name = category.value.trim();
        if (text && name) save([...rules, { match: text, category: name }]);
      }),
    );
  });

  const ruleRows = rules.map((rule, i) =>
    el(
      "div",
      "display: flex; gap: 8px; align-items: center; margin: 4px 0",
      el("span", "flex: 1", `“${rule.match}” → ${rule.category}`),
      button("Remove", () => save(rules.filter((_, j) => j !== i))),
    ),
  );

  root.replaceChildren(
    el(
      "p",
      "margin: 0 0 12px; color: var(--text-2)",
      `${money.format(spent)} left the accounts in view as ${rows.length} withdrawals.`,
    ),
    ...bars,
    asks.length ? heading("Not sorted yet") : null,
    ...asks,
    rules.length ? heading("Rules, first match wins") : null,
    ...ruleRows,
  );
});
