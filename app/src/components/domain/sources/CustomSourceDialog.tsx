import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { Trans, useLingui } from "@lingui/react/macro";
import { api } from "../../../lib/api";
import type { CustomSource } from "../../../lib/types";
import { Field, FormDialog, List, ListRow, Money, QueryError } from "../../ui";

// eslint-disable-next-line lingui/no-unlocalized-strings -- a URL scheme, not text
const HTTPS = "https://";
// eslint-disable-next-line lingui/no-unlocalized-strings -- template placeholders, not text
const PLACEHOLDERS = "{SYMBOL} {ISIN} {MIC} {CURRENCY} {FROM} {TO} {FROM:%s} {TO:%s} {KEY}";
// eslint-disable-next-line lingui/no-unlocalized-strings -- template placeholders, not text
const PAIR_PLACEHOLDERS = "{BASE} {QUOTE} {FROM} {TO} {FROM:%s} {TO:%s} {KEY}";
// eslint-disable-next-line lingui/no-unlocalized-strings -- a template placeholder, not text
const KEY = "{KEY}";

const EMPTY: CustomSource = {
  id: "",
  label: "",
  role: "quotes",
  url: HTTPS,
  headers: [],
  format: { kind: "json", date_path: "$[*].date", close_path: "$[*].close" },
  date_format: null,
  factor: null,
  currency: null,
};

/** A readable id from the name, fixed once the source exists: instruments store it. */
function slug(label: string): string {
  const base = label
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
  return `custom:${base || "source"}`;
}

/** Describe a feed, try it on one symbol, then save it. Nothing is stored before `Save`. */
export function CustomSourceDialog({
  source,
  onKey,
  onClose,
}: {
  source: CustomSource | null;
  onKey: (id: string) => void;
  onClose: () => void;
}) {
  const { t } = useLingui();
  const [draft, setDraft] = useState<CustomSource>(source ?? EMPTY);
  const [symbol, setSymbol] = useState("");
  const [currency, setCurrency] = useState("USD");
  const header = draft.headers[0] ?? { name: "", value: "" };
  const complete = { ...draft, id: source?.id ?? slug(draft.label) };

  const save = useMutation({ mutationFn: () => api.marketCustomSave(complete), onSuccess: onClose });
  const remove = useMutation({ mutationFn: () => api.marketCustomDelete(complete.id), onSuccess: onClose });
  const test = useMutation({ mutationFn: () => api.marketCustomTest(complete, symbol.trim(), currency) });

  const set = (patch: Partial<CustomSource>) => setDraft({ ...draft, ...patch });
  const [datePath, closePath] =
    draft.format.kind === "json"
      ? [draft.format.date_path, draft.format.close_path]
      : [draft.format.date_column, draft.format.close_column];
  const setPaths = (date: string, close: string) =>
    set({
      format:
        draft.format.kind === "json"
          ? { kind: "json", date_path: date, close_path: close }
          : { kind: "csv", date_column: date, close_column: close },
    });

  return (
    <FormDialog
      wide
      title={source ? t`Edit source` : t`New source`}
      onClose={onClose}
      onSubmit={() => save.mutate()}
      busy={save.isPending}
      error={save.error ?? remove.error}
      ready={draft.label.trim().length > 0 && draft.url.startsWith(HTTPS) && !!datePath && !!closePath}
      lead={
        source ? (
          <button
            type="button"
            className="btn btn--sm btn--danger"
            disabled={remove.isPending}
            onClick={() => remove.mutate()}
          >
            <Trans>Delete source</Trans>
          </button>
        ) : undefined
      }
    >
      <Field label={t`Name`}>
        <input value={draft.label} onChange={(e) => set({ label: e.target.value })} />
      </Field>
      <Field
        label={t`It answers`}
        options={[
          { value: "quotes", label: t`Prices of instruments` },
          { value: "fx", label: t`Exchange rates` },
        ]}
        value={draft.role}
        onChange={(role) => set({ role })}
      />
      <Field
        label={t`Address`}
        hint={`${t`https only. Placeholders:`} ${draft.role === "fx" ? PAIR_PLACEHOLDERS : PLACEHOLDERS}`}
      >
        <input value={draft.url} onChange={(e) => set({ url: e.target.value })} />
      </Field>
      <Field
        label={t`Answer format`}
        options={[
          { value: "json", label: "JSON" },
          { value: "csv", label: "CSV" },
        ]}
        value={draft.format.kind}
        onChange={(kind) =>
          set({
            format:
              kind === "json"
                ? { kind: "json", date_path: "$[*].date", close_path: "$[*].close" }
                : { kind: "csv", date_column: "Date", close_column: "Close" },
          })
        }
      />
      <Field
        label={draft.format.kind === "json" ? t`Path to the dates` : t`Date column`}
        hint={draft.format.kind === "json" ? t`For example $.values[*].datetime` : undefined}
      >
        <input value={datePath} onChange={(e) => setPaths(e.target.value, closePath)} />
      </Field>
      <Field label={draft.format.kind === "json" ? t`Path to the closes` : t`Close column`}>
        <input value={closePath} onChange={(e) => setPaths(datePath, e.target.value)} />
      </Field>
      <Field
        label={t`Date format`}
        hint={t`Empty reads ISO dates and Unix timestamps; otherwise e.g. %d.%m.%Y`}
      >
        <input
          value={draft.date_format ?? ""}
          onChange={(e) => set({ date_format: e.target.value || null })}
        />
      </Field>
      {draft.role === "quotes" && (
        <Field label={t`Currency of the closes`} hint={t`Empty means the instrument's own currency`}>
          <input
            value={draft.currency ?? ""}
            onChange={(e) => set({ currency: e.target.value.toUpperCase() || null })}
          />
        </Field>
      )}
      <Field label={t`Multiply closes by`} hint={t`0.01 for a feed that answers in cents`}>
        <input
          inputMode="decimal"
          value={draft.factor ?? ""}
          onChange={(e) => set({ factor: e.target.value.trim() || null })}
        />
      </Field>
      <Field
        label={t`Header name`}
        hint={t`Optional, e.g. Authorization; ${KEY} in its value is replaced by the saved key`}
      >
        <input
          value={header.name}
          onChange={(e) =>
            set({ headers: e.target.value ? [{ name: e.target.value, value: header.value }] : [] })
          }
        />
      </Field>
      {header.name && (
        <Field label={t`Header value`}>
          <input
            value={header.value}
            onChange={(e) => set({ headers: [{ name: header.name, value: e.target.value }] })}
          />
        </Field>
      )}
      {source && (
        <button type="button" className="btn btn--sm btn--ghost" onClick={() => onKey(complete.id)}>
          <Trans>Set key</Trans>
        </button>
      )}

      <Field
        label={draft.role === "fx" ? t`Try it with a pair` : t`Try it with a symbol`}
        hint={draft.role === "fx" ? t`For example EUR/USD` : undefined}
      >
        <input value={symbol} onChange={(e) => setSymbol(e.target.value)} />
      </Field>
      {draft.role === "quotes" && (
        <Field label={t`Instrument currency`}>
          <input value={currency} onChange={(e) => setCurrency(e.target.value.toUpperCase())} />
        </Field>
      )}
      <button
        type="button"
        className="btn btn--sm"
        disabled={!symbol.trim() || test.isPending}
        onClick={() => test.mutate()}
      >
        {test.isPending ? <Trans>Asking…</Trans> : <Trans>Test</Trans>}
      </button>
      {test.error && <QueryError error={test.error} />}
      {test.data && (
        <List>
          {test.data.length === 0 ? (
            <ListRow title={t`The source answered, but nothing in the last 30 days`} />
          ) : (
            test.data.map((row) => (
              <ListRow
                key={row.date}
                title={row.date}
                value={<Money value={row.close} currency={row.currency} />}
              />
            ))
          )}
        </List>
      )}
    </FormDialog>
  );
}
