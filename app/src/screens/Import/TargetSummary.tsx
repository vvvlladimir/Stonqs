import type { I18n } from "@lingui/core";
import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { Buttons, Choice, List, ListRow, Panel } from "../../components/ui";
import { accountKindLabel } from "../../lib/kinds";
import type { AccountRow, ImportMapping, ImportPreviewData, ParseConfig } from "../../lib/types";
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

function accountLabel(i18n: I18n, account: AccountRow): string {
  return `${accountKindLabel(i18n, account.kind)} · ${account.name} · ${account.currency}`;
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
  // A securities account never holds the money: its cash leg is the account it references.
  const cash =
    target?.kind === "SECURITIES"
      ? (accounts.find((a) => a.id === target.reference_account_id) ?? null)
      : target;

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
      <List>
        <ListRow
          title={t`Default account`}
          sub={t`Where rows without an account column land`}
          end={
            <Choice
              wide
              label={t`Default account`}
              placeholder={t`— none chosen —`}
              value={mapping.account_id ?? ""}
              onChange={(id) => onChange({ ...mapping, account_id: id || null })}
              options={accounts.map((a) => ({ value: a.id, label: accountLabel(i18n, a) }))}
            />
          }
        />
        <ListRow
          title={t`Cash account`}
          sub={t`Money lands here: deposits, dividends, fees`}
          value={cash ? accountLabel(i18n, cash) : "—"}
        />
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
