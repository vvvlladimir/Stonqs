import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../lib/api";
import { formatDay } from "../../lib/format";
import { amountOf } from "../../lib/plans";
import { affects, useInvalidate, usePlanDue } from "../../lib/queries";
import { Logo } from "../../components/domain/Instrument";
import { Async, Banner, DataTable, Empty, ErrorText, Modal, Money, Panel } from "../../components/ui";
import type { PlanDue, PlanRow, Transaction, TransactionInput } from "../../lib/types";

/** Edits kept per occurrence date; an untouched occurrence is committed exactly as proposed. */
type Edits = Record<string, TransactionInput[]>;

function toInput(draft: Transaction): TransactionInput {
  return {
    id: null,
    account_id: draft.account_id,
    security_id: draft.security_id,
    kind: draft.kind,
    date: draft.date,
    quantity: draft.quantity,
    price: draft.price,
    amount: draft.amount,
    fees: draft.fees,
    taxes: draft.taxes,
    currency: draft.currency,
    fee_currency: draft.fee_currency,
    tax_currency: draft.tax_currency,
    fx_rate_to_base: draft.fx_rate_to_base,
    note: draft.note,
  };
}

/** A row is writable when it moves something: shares for a purchase, money for a contribution. */
const writable = (draft: TransactionInput) => amountOf(draft.quantity) !== 0 || amountOf(draft.amount) !== 0;

/**
 * What a plan still owes, one panel per occurrence. Nothing is written until Record is pressed
 * on that occurrence: the rows are a proposal, and the numbers the broker actually filled can
 * be typed over them first (ADR-0033).
 */
export function DuePanel({ row, onClose }: { row: PlanRow; onClose: () => void }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const due = usePlanDue(row.plan.id);
  const [edits, setEdits] = useState<Edits>({});

  const commit = useMutation({
    mutationFn: ({ date, drafts }: { date: string; drafts: TransactionInput[] }) =>
      api.planCommit(row.plan.id, date, drafts),
    onSuccess: () => invalidate(...affects.transactions),
  });

  // The edit seeds itself from the rows on screen: an untouched occurrence has no entry yet,
  // and starting from an empty list would drop every row the user did not type into.
  const setDraft = (
    date: string,
    base: TransactionInput[],
    index: number,
    patch: Partial<TransactionInput>,
  ) =>
    setEdits((current) => ({
      ...current,
      [date]: (current[date] ?? base).map((d, i) => (i === index ? { ...d, ...patch } : d)),
    }));

  return (
    <Modal title={t`Contributions due · ${row.plan.name}`} onClose={onClose} wide>
      <ErrorText error={commit.error} />
      <Async
        query={due}
        empty={
          <Empty title={t`Nothing owed`}>
            <Trans>Every contribution up to today has been recorded.</Trans>
          </Empty>
        }
      >
        {(items) => (
          <div className="stack">
            {items.map((item) => (
              <Occurrence
                key={item.date}
                item={item}
                drafts={edits[item.date] ?? item.drafts.map(toInput)}
                busy={commit.isPending}
                onEdit={(index, patch, base) => setDraft(item.date, base, index, patch)}
                onRecord={(drafts) => commit.mutate({ date: item.date, drafts })}
              />
            ))}
          </div>
        )}
      </Async>
    </Modal>
  );
}

/** One firing of the plan: the rows it would write, with what it cannot spend beneath them. */
function Occurrence({
  item,
  drafts,
  busy,
  onEdit,
  onRecord,
}: {
  item: PlanDue;
  drafts: TransactionInput[];
  busy: boolean;
  onEdit: (index: number, patch: Partial<TransactionInput>, base: TransactionInput[]) => void;
  onRecord: (drafts: TransactionInput[]) => void;
}) {
  const { t } = useLingui();
  const ready = drafts.length > 0 && drafts.every(writable);
  const edit = (draft: TransactionInput, patch: Partial<TransactionInput>) =>
    onEdit(drafts.indexOf(draft), patch, drafts);

  return (
    <Panel
      title={formatDay(item.date)}
      note={
        drafts.length > 0
          ? plural(drafts.length, { one: "# transaction", other: "# transactions" })
          : undefined
      }
      tools={
        <button className="btn" disabled={!ready || busy} onClick={() => onRecord(drafts)}>
          <Trans>Record</Trans>
        </button>
      }
      table={drafts.length > 0}
    >
      {item.problem === "MISSING_PRICE" && (
        <Banner tone="warn">
          <Trans>
            No quote for this date yet, so there is nothing to propose. Refresh market data, or enter the
            purchase by hand.
          </Trans>
        </Banner>
      )}
      {item.problem === "MISSING_RATE" && (
        <Banner tone="warn">
          <Trans>No exchange rate for this date yet, so the contribution cannot be converted.</Trans>
        </Banner>
      )}
      {item.occurrence && drafts.length === 0 && (
        <Banner tone="info">
          <Trans>The contribution does not reach one tradable unit, so it stays as cash.</Trans>
        </Banner>
      )}

      {drafts.length > 0 && (
        <DataTable
          card={false}
          rows={drafts}
          rowKey={(draft) => `${draft.security_id ?? "cash"}:${draft.currency}`}
          columns={[
            {
              key: "instrument",
              header: t`Instrument`,
              align: "left",
              cell: (draft) => {
                const trade = item.occurrence?.trades[drafts.indexOf(draft)];
                return (
                  <div className="instr">
                    <Logo symbol={trade?.symbol ?? ""} name={draft.currency} />
                    <div>
                      <div className="nm">{trade?.symbol ?? t`Cash contribution`}</div>
                      <div className="sub">{draft.currency}</div>
                    </div>
                  </div>
                );
              },
            },
            {
              key: "quantity",
              header: t`Quantity`,
              width: "7rem",
              clamp: false,
              cell: (draft) => (
                <input
                  inputMode="decimal"
                  aria-label={t`Quantity`}
                  value={draft.quantity ?? ""}
                  onChange={(e) => edit(draft, { quantity: e.target.value })}
                />
              ),
            },
            {
              key: "price",
              header: t`Price`,
              width: "8rem",
              clamp: false,
              cell: (draft) => (
                <input
                  inputMode="decimal"
                  aria-label={t`Price`}
                  value={draft.price ?? ""}
                  onChange={(e) => edit(draft, { price: e.target.value })}
                />
              ),
            },
            {
              key: "fees",
              header: t`Commission`,
              width: "7rem",
              only: "wide",
              clamp: false,
              cell: (draft) => (
                <input
                  inputMode="decimal"
                  aria-label={t`Commission`}
                  value={draft.fees ?? ""}
                  onChange={(e) => edit(draft, { fees: e.target.value })}
                />
              ),
            },
            {
              key: "amount",
              header: t`Amount`,
              className: "money",
              // Recomputed from the fields on screen, so a typed-over fill shows its own total.
              cell: (draft) => (
                <>
                  <Money value={String(amountOf(draft.quantity) * amountOf(draft.price))} digits={2} />{" "}
                  <span className="dim">{draft.currency}</span>
                </>
              ),
            },
          ]}
        />
      )}

      {item.occurrence && amountOf(item.occurrence.cash_left) !== 0 && (
        <p className="panel__note">
          <Trans>
            <Money value={item.occurrence.cash_left} currency={item.occurrence.currency} digits={2} /> of{" "}
            <Money value={item.occurrence.amount} currency={item.occurrence.currency} digits={2} /> stays as
            cash — it does not reach a whole unit
          </Trans>
        </p>
      )}
    </Panel>
  );
}
