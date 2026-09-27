import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { usePositionReturn } from "../../../lib/queries";
import { Money, Panel, Percent, Quantity, SkeletonRows, Tag } from "../../ui";
import { formatDecimal, formatQuantity, signOf, toneClass } from "../../../lib/format";
import type { PositionRow, SecurityRow } from "../../../lib/types";
/** Facts a held position ends up printing; the placeholder reserves exactly that many lines. */
const FACTS_WHILE_LOADING = 12;

type Returns = ReturnType<typeof usePositionReturn>;

/** What is true of the position, then of the instrument itself. */
export function FactsPanel({
  security,
  row,
  currency,
  returns,
  loading,
}: {
  security: SecurityRow | undefined;
  row: PositionRow | undefined;
  currency: string;
  returns: Returns;
  loading: boolean;
}) {
  const { t, i18n } = useLingui();
  return (
    <Panel title={t`Facts`}>
      {loading && <SkeletonRows rows={FACTS_WHILE_LOADING} />}
      {!loading && (
        <dl className="facts">
          {row && (
            <>
              <Fact k={t`Quantity`} v={<Quantity value={row.quantity} />} />
              <Fact
                k={t`Price`}
                v={
                  <>
                    <Money value={row.price} /> {row.currency}
                  </>
                }
              />
              <Fact k={t`Cost basis`} v={<Money value={row.cost_basis_base} currency={currency} />} />
              <Fact
                k={t`Unrealized`}
                v={<Money value={row.unrealized_pnl_base} signed tone={false} />}
                tone={toneClass(row.unrealized_pnl_base)}
              />
              <Fact
                k={t`Realized`}
                v={
                  signOf(row.realized_pnl_base) === "zero" ? (
                    t`nothing sold`
                  ) : (
                    <Money value={row.realized_pnl_base} signed tone={false} />
                  )
                }
                tone={toneClass(row.realized_pnl_base)}
              />
              <Fact
                k={t`Dividends`}
                v={
                  signOf(row.dividends_base) === "zero" ? (
                    t`pays none`
                  ) : (
                    <Money value={row.dividends_base} currency={currency} />
                  )
                }
              />
              <Fact k="IRR" v={returns.data?.xirr ? <Percent value={returns.data.xirr} signed /> : "—"} />
            </>
          )}
          <Fact k="ISIN" v={security?.isin ?? "—"} />
          {security?.wkn && <Fact k="WKN" v={security.wkn} />}
          <Fact k={t`Venue`} v={listingOf(i18n, security)} />
          <Fact k={t`Quote source`} v={security?.data_source ?? <Tag warn>{t`manual`}</Tag>} />
          <Fact k={t`Quote history`} v={historyOf(i18n, security)} />
          <Fact k={t`Quantity step`} v={stepOf(i18n, security)} />
          {row && <Fact k={t`Held in`} v={row.accounts.map((a) => a.account_name).join(", ") || "—"} />}
        </dl>
      )}
    </Panel>
  );
}

function Fact({ k, v, tone }: { k: string; v: React.ReactNode; tone?: string }) {
  return (
    <>
      <dt>{k}</dt>
      <dd className={tone}>{v}</dd>
    </>
  );
}

function listingOf(i18n: I18n, security?: SecurityRow): React.ReactNode {
  if (!security) return "—";
  if (!security.mic) return <Tag warn>{i18n._(msg`not chosen`)}</Tag>;
  return [security.venue, security.mic, security.quote_currency ?? security.currency]
    .filter(Boolean)
    .join(" · ");
}

function historyOf(i18n: I18n, security?: SecurityRow): string {
  if (!security || security.quote_count === 0 || !security.coverage_from || !security.coverage_to) {
    return i18n._(msg`no quotes`);
  }
  const count = formatDecimal(String(security.quote_count), { digits: 0 });
  return `${security.coverage_from} — ${security.coverage_to} · ${count}`;
}

/** Shows the effective quantity step and its source. */
function stepOf(i18n: I18n, security?: SecurityRow): string {
  if (!security) return "—";
  const effective = formatQuantity(security.effective_quantity_step);
  if (security.quantity_step !== null) return effective;
  return security.quantity_step_observed !== null
    ? i18n._(msg`${effective} (from trades)`)
    : i18n._(msg`${effective} (from the kind)`);
}
