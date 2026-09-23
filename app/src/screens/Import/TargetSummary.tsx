import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Buttons, List, ListRow, Panel } from "../../components/ui";
import type { AccountRow, ImportMapping, ImportPreviewData, ParseConfig } from "../../lib/types";
import { AccountTarget } from "./AccountTarget";
import { ParseSettings } from "./ParseSettings";
import { SIGN_LABELS } from "./labels";

/** Dates of the rows that parsed; an unparsed file simply has no span to state. */
function span(preview: ImportPreviewData): string | null {
  const dates = preview.rows.map((row) => row.draft?.date).filter(Boolean) as string[];
  if (dates.length === 0) return null;
  const from = dates.reduce((a, b) => (a < b ? a : b));
  const to = dates.reduce((a, b) => (a > b ? a : b));
  return from === to ? from : `${from} — ${to}`;
}

/**
 * Where this file is about to land, stated before the mapping work starts: the whole
 * step is answering "is this right", and that question needs its subject first.
 */
export function TargetSummary({
  preview,
  mapping,
  config,
  accounts,
  onChange,
  onConfig,
}: {
  preview: ImportPreviewData;
  mapping: ImportMapping;
  config: ParseConfig;
  accounts: AccountRow[];
  onChange: (mapping: ImportMapping) => void;
  onConfig: (config: ParseConfig, mapping: ImportMapping) => void;
}) {
  const { t, i18n } = useLingui();
  const [settings, setSettings] = useState(false);

  const target = accounts.find((a) => a.id === mapping.account_id) ?? null;

  const currency = mapping.default_currency
    ? mapping.default_currency
    : mapping.columns.CURRENCY
      ? t`from column "${mapping.columns.CURRENCY}"`
      : (target?.currency ?? t`not set`);

  const dates = span(preview);

  return (
    <Panel
      title={t`What is written, and where`}
      note={plural(preview.rows.length, { one: "# row", other: "# rows" })}
      tools={
        <Buttons>
          <button className="iconbtn iconbtn--sm" onClick={() => setSettings(!settings)}>
            {settings ? t`collapse` : t`how the file is read`}
          </button>
        </Buttons>
      }
    >
      <AccountTarget accounts={accounts} mapping={mapping} onChange={onChange} />
      <List>
        <ListRow title={t`Transaction currency`} value={currency} />
        {dates && <ListRow title={t`Export period`} value={dates} />}
        <ListRow
          title={t`How the file is read`}
          value={t`delimiter "${config.delimiter ?? "?"}" · dates ${config.date_format ?? t`not detected`} · ${i18n._(SIGN_LABELS[preview.amount_sign])}`}
        />
      </List>
      {settings && (
        <ParseSettings
          config={config}
          mapping={mapping}
          detected={preview.amount_sign}
          detectedBasis={preview.amount_basis}
          onChange={onConfig}
        />
      )}
    </Panel>
  );
}
