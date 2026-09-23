import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { Badge, Buttons, ErrorText, List, ListRow, Money, Panel } from "../../components/ui";
import { useInvalidate, useTransferSuggestions, affects } from "../../lib/queries";
import type { TransferSuggestion } from "../../lib/types";

/**
 * Moves that arrived as two halves from two exports. Offered after the write, never applied by
 * itself: two amounts agreeing is not proof that one payment is the other, and linking the wrong
 * pair erases a real deposit and a real withdrawal from every return figure at once.
 */
export function TransferPairs({ enabled }: { enabled: boolean }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const suggestions = useTransferSuggestions(enabled);

  const link = useMutation({
    mutationFn: (pair: TransferSuggestion) => api.transferLink([pair.out_id, pair.in_id]),
    onSuccess: () => invalidate(...affects.transactions),
  });

  const found = suggestions.data ?? [];
  if (found.length === 0) return null;

  return (
    <Panel
      title={t`Money that left one account and arrived at another`}
      note={plural(found.length, { one: "# pair", other: "# pairs" })}
    >
      <p className="muted">
        <Trans>
          Each of these is a payment out of one account and a payment into another, of the same size within
          days. If it is one move — a portfolio carried between brokers, money sent to your own account — join
          the two: the app then stops reading it as money leaving the portfolio and coming back, which
          distorts every return figure. If they are genuinely two separate payments, leave them alone.
        </Trans>
      </p>
      {link.error && <ErrorText>{String(link.error)}</ErrorText>}
      <List>
        {found.map((pair) => (
          <ListRow
            key={`${pair.out_id}:${pair.in_id}`}
            box
            wrap
            title={`${pair.account_out_name} → ${pair.account_in_name}`}
            sub={pair.date_out === pair.date_in ? pair.date_out : `${pair.date_out} → ${pair.date_in}`}
            value={
              <>
                <Money value={pair.amount_out} />
                <span className="cur">{pair.currency}</span>
              </>
            }
            meta={
              pair.amount_out === pair.amount_in ? undefined : (
                <Badge tone="warn">
                  <Trans>arrived {pair.amount_in}</Trans>
                </Badge>
              )
            }
            end={
              <Buttons>
                <button className="btn btn--sm" disabled={link.isPending} onClick={() => link.mutate(pair)}>
                  <Trans>One move</Trans>
                </button>
              </Buttons>
            }
          />
        ))}
      </List>
    </Panel>
  );
}
