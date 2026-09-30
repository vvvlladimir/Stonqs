import { Trans, useLingui } from "@lingui/react/macro";
import { PlusIcon } from "@phosphor-icons/react";
import { Empty, List, Panel } from "../../components/ui";
import { isOwedKind } from "../../lib/kinds";
import type { Asset, NetWorth } from "../../lib/types";
import { AssetCard } from "./AssetCard";

/** One side of the balance: the things owned, or the debts. */
export function Side({
  side,
  reading,
  assets,
  onAdd,
  onValues,
  onEdit,
  onDelete,
}: {
  side: "owned" | "owed";
  reading: NetWorth;
  assets: Asset[];
  onAdd: () => void;
  onValues: (asset: Asset) => void;
  onEdit: (asset: Asset) => void;
  onDelete: (asset: Asset) => void;
}) {
  const { t } = useLingui();
  const owed = side === "owed";
  const mine = assets.filter((asset) => isOwedKind(asset.kind) === owed);
  // The reading order is by size; an asset with no figure yet has no place in it and goes last.
  const order = reading.holdings.map((holding) => holding.asset_id);
  const sorted = [...mine].sort((a, b) => rank(order, a.id) - rank(order, b.id));

  return (
    <Panel
      title={owed ? t`Owed` : t`Owned`}
      info={
        owed
          ? t`Debts, entered as what is still owed. Subtracted from net worth.`
          : t`Things with no market to price them. Each carries the figures you write yourself.`
      }
      tools={
        <button type="button" className="btn btn--sm" onClick={onAdd}>
          <PlusIcon /> {owed ? <Trans>Add debt</Trans> : <Trans>Add asset</Trans>}
        </button>
      }
    >
      {sorted.length === 0 ? (
        <Empty title={owed ? t`No debts` : t`Nothing yet`}>
          {owed ? (
            <Trans>
              A mortgage, a loan, a credit card. Its rate and payment describe what is ahead; what is owed
              today is the figure you write.
            </Trans>
          ) : (
            <Trans>
              A flat, a car, something valuable, money held where the app has no ledger. None of it has a
              return — it only makes net worth complete.
            </Trans>
          )}
        </Empty>
      ) : (
        <List variant="cards">
          {sorted.map((asset) => (
            <AssetCard
              key={asset.id}
              asset={asset}
              holding={reading.holdings.find((h) => h.asset_id === asset.id)}
              base={reading.base_currency}
              securedByName={assets.find((a) => a.id === asset.secured_by)?.name}
              onValues={() => onValues(asset)}
              onEdit={() => onEdit(asset)}
              onDelete={() => onDelete(asset)}
            />
          ))}
        </List>
      )}
    </Panel>
  );
}

function rank(order: string[], id: string): number {
  const at = order.indexOf(id);
  return at < 0 ? order.length : at;
}
