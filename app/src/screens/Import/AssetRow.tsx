import { Plural, Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { Badge, Buttons, ErrorText, Field, FormDialog, ListRow } from "../../components/ui";
import { SECURITY_KINDS, securityKindLabel } from "../../lib/kinds";
import type { SecurityDraft, SymbolMapping } from "../../lib/types";
import { AssetSearch } from "./AssetSearch";
import { normalizeAlias } from "./labels";

/**
 * One broker code and the instrument it will become. Edits save as they are typed,
 * so the dialog's button only closes it — there is nothing left to confirm.
 */
export function AssetRow({
  symbol,
  aliases,
  onAlias,
  onPlan,
}: {
  symbol: SymbolMapping;
  aliases: Record<string, string>;
  onAlias: (from: string, to: string) => void;
  onPlan: (value: string, draft: SecurityDraft | null) => void;
}) {
  const { t, i18n } = useLingui();
  const [open, setOpen] = useState(false);
  const inDatabase = Boolean(symbol.security_id);
  const plan = symbol.planned;

  const resolve = useMutation({
    mutationFn: () =>
      api.importResolveSymbol(symbol.resolved, symbol.isin, symbol.file_name, symbol.currency),
    onSuccess: (draft) => draft && onPlan(symbol.value, draft),
  });

  return (
    <>
      <ListRow
        wrap
        title={<span className="mono">{plan ? plan.symbol : symbol.value}</span>}
        sub={
          <span className="inline">
            <span>{symbol.name ?? symbol.file_name ?? t`instrument not identified`}</span>
            {plan && <Badge tone="info">{plan.kind}</Badge>}
            {plan && <Badge>{plan.currency}</Badge>}
            {plan?.exchange && <Badge>{plan.exchange}</Badge>}
            {inDatabase && <Badge tone="in">{t`already in the database`}</Badge>}
            {plan && (
              <span className="dim">
                <Trans>from the file: {symbol.value}</Trans>
              </span>
            )}
          </span>
        }
        meta={<Plural value={symbol.count} one="# row" other="# rows" />}
        end={
          <Buttons>
            {!inDatabase && (
              <button
                className="iconbtn iconbtn--sm"
                disabled={resolve.isPending}
                onClick={() => resolve.mutate()}
              >
                {resolve.isPending ? t`searching…` : plan ? t`search again` : t`identify`}
              </button>
            )}
            <button className="iconbtn iconbtn--sm" onClick={() => setOpen(true)}>
              <Trans>edit</Trans>
            </button>
          </Buttons>
        }
        foot={resolve.error ? <ErrorText error={resolve.error} as="div" /> : undefined}
      />

      {open && (
        <FormDialog
          wide
          title={t`Instrument "${symbol.value}"`}
          onClose={() => setOpen(false)}
          onSubmit={() => setOpen(false)}
          submitLabel={t`Done`}
          busyLabel={t`Done`}
          lead={
            plan ? (
              <button
                type="button"
                className="btn btn--ghost btn--danger"
                onClick={() => onPlan(symbol.value, null)}
              >
                <Trans>Forget the match</Trans>
              </button>
            ) : undefined
          }
        >
          {plan && (
            <>
              <Field label={t`Ticker`} hint={t`The instrument appears in the portfolio under this name`}>
                <input
                  value={plan.symbol}
                  onChange={(e) => onPlan(symbol.value, { ...plan, symbol: e.target.value.toUpperCase() })}
                />
              </Field>
              <Field label={t`Name`}>
                <input
                  value={plan.name}
                  onChange={(e) => onPlan(symbol.value, { ...plan, name: e.target.value })}
                />
              </Field>
              <Field label={t`Currency`}>
                <input
                  value={plan.currency}
                  maxLength={3}
                  onChange={(e) => onPlan(symbol.value, { ...plan, currency: e.target.value.toUpperCase() })}
                />
              </Field>
              <Field
                label={t`Kind`}
                options={SECURITY_KINDS.map((kind) => ({
                  value: kind,
                  label: securityKindLabel(i18n, kind),
                }))}
                value={plan.kind}
                onChange={(kind) => onPlan(symbol.value, { ...plan, kind })}
              />
              <Field label="ISIN">
                <input
                  value={plan.isin ?? ""}
                  onChange={(e) => onPlan(symbol.value, { ...plan, isin: e.target.value || null })}
                />
              </Field>
            </>
          )}

          {!inDatabase && <AssetSearch symbol={symbol} onPick={onPlan} />}

          <Field
            label={t`Link to an instrument in the database`}
            hint={t`Type its ticker — the file's rows go to the existing instrument and no new one is created`}
          >
            <input
              value={aliases[normalizeAlias(symbol.value)] ?? ""}
              placeholder={t`ticker in the database`}
              onChange={(e) => onAlias(symbol.value, e.target.value)}
            />
          </Field>
        </FormDialog>
      )}
    </>
  );
}
