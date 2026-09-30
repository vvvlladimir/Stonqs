import { Trans, useLingui } from "@lingui/react/macro";
import { useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { PlusIcon, TrashIcon } from "@phosphor-icons/react";
import { api, today } from "../../lib/api";
import { formatDay } from "../../lib/format";
import { affects, useAssetValues, useInvalidate } from "../../lib/queries";
import { Async, Empty, ErrorText, Field, FieldPair, List, ListRow, Modal, Money } from "../../components/ui";
import type { Asset } from "../../lib/types";

/** Every figure ever written for one thing, and the form that adds another. */
export function ValueHistory({ asset, onClose }: { asset: Asset; onClose: () => void }) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const values = useAssetValues(asset.id);
  const [date, setDate] = useState(today());
  const [amount, setAmount] = useState("");

  const save = useMutation({
    mutationFn: api.assetValueSave,
    onSuccess: () => {
      setAmount("");
      invalidate(...affects.assets);
    },
  });
  const remove = useMutation({
    mutationFn: ({ date }: { date: string }) => api.assetValueDelete(asset.id, date),
    onSuccess: () => invalidate(...affects.assets),
  });

  return (
    <Modal title={t`${asset.name} · value history`} onClose={onClose}>
      <div className="stack">
        <FieldPair>
          <Field label={t`Day`}>
            <input type="date" value={date} onChange={(e) => setDate(e.target.value)} />
          </Field>
          <Field
            label={t`Value in ${asset.currency}`}
            hint={t`The same day answered twice replaces that day's figure.`}
          >
            <input inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value)} />
          </Field>
        </FieldPair>
        <button
          type="button"
          className="btn"
          disabled={amount.trim() === "" || save.isPending}
          onClick={() =>
            save.mutate({ asset_id: asset.id, date, amount, note: null })
          }
        >
          <PlusIcon /> <Trans>Add a valuation</Trans>
        </button>
        <ErrorText error={save.error ?? remove.error} />

        <Async
          query={values}
          empty={
            <Empty title={t`No figure yet`}>
              <Trans>
                Until it has one, this thing is absent from net worth — which is not the same as being
                worth nothing.
              </Trans>
            </Empty>
          }
        >
          {(rows) => (
            <List>
              {[...rows].reverse().map((row) => (
                <ListRow
                  key={row.date}
                  title={formatDay(row.date)}
                  value={<Money value={row.amount} currency={asset.currency} />}
                  sub={row.note ?? undefined}
                  end={
                    <button
                      type="button"
                      className="iconbtn iconbtn--sm iconbtn--danger"
                      aria-label={t`Delete`}
                      onClick={() => remove.mutate({ date: row.date })}
                    >
                      <TrashIcon />
                    </button>
                  }
                />
              ))}
            </List>
          )}
        </Async>
      </div>
    </Modal>
  );
}
