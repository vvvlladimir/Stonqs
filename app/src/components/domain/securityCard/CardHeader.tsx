import { Trans, useLingui } from "@lingui/react/macro";
import { usePositionReturn } from "../../../lib/queries";
import { Logo } from "../Instrument";
import { Metric, Money, Num, Percent, Skeleton, Tag } from "../../ui";
import { signOf } from "../../../lib/format";
import { securityKindLabel } from "../../../lib/kinds";
import type { PositionRow, SecurityRow } from "../../../lib/types";
type Returns = ReturnType<typeof usePositionReturn>;

/** Who the instrument is and, when it is held, the four figures of the position. */
export function CardHeader({
  symbol,
  name,
  security,
  row,
  currency,
  returns,
  loading,
}: {
  symbol: string;
  name: string;
  security: SecurityRow | undefined;
  row: PositionRow | undefined;
  currency: string;
  returns: Returns;
  loading: boolean;
}) {
  const { t, i18n } = useLingui();
  return (
    <>
      <div className="secpop__top">
        <Logo symbol={symbol} name={name} />
        <div className="min0">
          <div className="secpop__t">{symbol}</div>
          <div className="row__sub">
            {loading ? (
              <Skeleton w="12rem" h="0.9rem" />
            ) : (
              <>
                {security?.isin && <Num>{security.isin}</Num>}
                <span>{security?.venue ?? t`no venue chosen`}</span>
                {security && <Tag>{securityKindLabel(i18n, security.kind)}</Tag>}
              </>
            )}
          </div>
        </div>
        <span className="spacer" />
        {loading && (
          <div className="secpop__v">
            <Skeleton w="7rem" h="1.6rem" />
            <div>
              <Skeleton w="4rem" h="0.9rem" />
            </div>
          </div>
        )}
        {!loading && row && (
          <div className="secpop__v">
            <Money value={row.market_value_base} currency={currency} />
            {row.day_change !== null && (
              <div>
                <Trans>
                  <Percent value={row.day_change} signed /> today
                </Trans>
              </div>
            )}
          </div>
        )}
      </div>

      {loading ? (
        <div className="secpop__grid">
          {[t`Share of the source`, t`Return`, t`Unrealized`, t`Dividends`].map((label) => (
            <Metric key={label} label={label} value={<Skeleton w="4.5rem" h="1.3rem" />} />
          ))}
        </div>
      ) : row ? (
        <div className="secpop__grid">
          <Metric label={t`Share of the source`} value={<Percent value={row.weight} digits={1} />} />
          <Metric
            label={t`Return`}
            value={
              returns.isPending ? (
                <Skeleton w="4.5rem" h="1.3rem" />
              ) : returns.data?.twr ? (
                <Percent value={returns.data.twr} signed digits={1} tone={false} />
              ) : (
                "—"
              )
            }
            hint={t`TWR since the first purchase`}
            tone={returns.data?.twr ? signOf(returns.data.twr) : "neutral"}
          />
          <Metric
            label={t`Unrealized`}
            value={<Money value={row.unrealized_pnl_base} signed tone={false} />}
            tone={signOf(row.unrealized_pnl_base)}
          />
          <Metric label={t`Dividends`} value={<Money value={row.dividends_base} />} hint={t`all time`} />
        </div>
      ) : (
        <p className="panel__note secpop__none">
          <Trans>Nothing is held in this instrument now — what follows is its own history.</Trans>
        </p>
      )}
    </>
  );
}
