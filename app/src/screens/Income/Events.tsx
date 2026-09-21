import { Trans, useLingui } from "@lingui/react/macro";
import { DayMark, List, ListRow, Money } from "../../components/ui";
import { KindTag } from "../../components/domain/KindTag";
import { transactionLabel } from "../../lib/kinds";
import type { IncomeEvent } from "../../lib/types";
import { SecurityLink } from "../../components/domain/SecurityCardProvider";

export function Events({ events, currency }: { events: IncomeEvent[]; currency: string }) {
  const { i18n } = useLingui();
  if (events.length === 0)
    return (
      <p className="muted">
        <Trans>No payments.</Trans>
      </p>
    );

  // Show the newest income events first.
  const sorted = [...events].sort((a, b) => b.date.localeCompare(a.date));

  return (
    <List>
      {sorted.map((event, i) => {
        const label = transactionLabel(i18n, event.kind);
        const named = Boolean(event.symbol || event.name);
        return (
          <ListRow
            key={`${event.date}-${event.security_id ?? "cash"}-${i}`}
            lead={<DayMark date={event.date} />}
            title={<SecurityLink id={event.security_id}>{event.name || event.symbol || label}</SecurityLink>}
            sub={
              <>
                {/* An account payment has no ticker, so its kind is the only subtitle. */}
                {named ? (
                  <span>{event.symbol}</span>
                ) : (
                  <span>
                    <Trans>on the account</Trans>
                  </span>
                )}
                {named && <KindTag kind={event.kind} />}
                {event.currency !== currency && (
                  <Money value={event.gross_in_currency} currency={event.currency} />
                )}
              </>
            }
            value={<Money value={event.net_base} currency={currency} signed />}
            meta={
              Number(event.taxes_base) !== 0 ? (
                <>
                  <Trans>
                    <Money value={event.gross_base} currency={currency} /> accrued
                  </Trans>
                </>
              ) : undefined
            }
          />
        );
      })}
    </List>
  );
}
