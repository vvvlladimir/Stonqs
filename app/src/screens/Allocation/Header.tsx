import { bucketLabel } from "../../lib/taxonomy";
import { Trans, useLingui } from "@lingui/react/macro";
import { CheckIcon, TreeStructureIcon } from "@phosphor-icons/react";
import { Banner, Empty, Metric, Metrics, Money, Percent } from "../../components/ui";
import type { Allocation, AllocationBucket, InstalledTaxonomySet } from "../../lib/types";
import type { useAllocationDialogs } from "./Dialogs";
import type { LevelRow } from "./model";

/** Everything above the panels: the unclassified warning and the four level metrics. */
export function UnclassifiedBanner({
  taxonomyName,
  unclassified,
  currency,
  onShow,
}: {
  taxonomyName: string;
  unclassified: AllocationBucket;
  currency: string;
  onShow: () => void;
}) {
  if (!(Number(unclassified.weight) > 0.005)) {
    return (
      <Banner tone="info">
        <Trans>Every instrument is classified under "{taxonomyName}".</Trans>
      </Banner>
    );
  }
  return (
    <Banner
      action={
        <button type="button" className="btn btn--sm" onClick={onShow}>
          <Trans>Show them</Trans>
        </button>
      }
    >
      <b>
        <Percent value={unclassified.weight} digits={1} />
      </b>{" "}
      <Trans>
        of the portfolio (<Money value={unclassified.value_base} currency={currency} />) is not classified
        under "{taxonomyName}"
      </Trans>
      ».
    </Banner>
  );
}

export function AllocationMetrics({
  allocation,
  here,
  parts,
  unclassified,
  currency,
  taxonomyName,
  atPositions,
}: {
  allocation: Allocation | undefined;
  here: AllocationBucket | null;
  parts: LevelRow[];
  unclassified: AllocationBucket | undefined;
  currency: string;
  taxonomyName: string | undefined;
  atPositions: boolean;
}) {
  const { t, i18n } = useLingui();
  const largest = [...parts].sort((a, b) => Number(b.weight) - Number(a.weight))[0];
  const gap = unclassified && Number(unclassified.weight) > 0.005;

  return (
    <Metrics>
      <Metric
        label={t`Value`}
        value={
          here ? (
            <Money value={here.value_base} currency={currency} />
          ) : allocation ? (
            <Money value={allocation.total_base} currency={currency} />
          ) : (
            "…"
          )
        }
        hint={here ? t`inside "${bucketLabel(i18n, here)}"` : t`base currency ${currency}`}
        tip={t`Value of the current selection at the valuation date.`}
      />
      <Metric
        label={atPositions ? t`Instruments on this level` : t`Categories on this level`}
        value={allocation ? String(parts.length) : "…"}
        hint={here ? t`inside "${bucketLabel(i18n, here)}"` : (taxonomyName ?? "—")}
        tip={t`How many parts this level is split into.`}
      />
      <Metric
        label={t`Unclassified`}
        value={unclassified ? <Percent value={unclassified.weight} digits={1} /> : "—"}
        tone={gap ? "negative" : "neutral"}
        hint={gap ? t`needs a decision` : t`nothing left over`}
        tip={t`Share of the portfolio with no category in this tree.`}
      />
      <Metric
        label={t`Largest share`}
        value={largest ? <Percent value={largest.weight} digits={1} /> : "—"}
        hint={largest ? bucketLabel(i18n, largest) : t`no data`}
        tip={t`The heaviest part of this level.`}
      />
    </Metrics>
  );
}

export function EditTreeToggle({ editing, onToggle }: { editing: boolean; onToggle: () => void }) {
  return (
    <button type="button" className={`iconbtn${editing ? " iconbtn--on" : ""}`} onClick={onToggle}>
      {editing ? (
        <>
          <CheckIcon /> <Trans>Done</Trans>
        </>
      ) : (
        <>
          <TreeStructureIcon /> <Trans>Edit tree</Trans>
        </>
      )}
    </button>
  );
}

/** The screen before any tree exists: start one, read one from a file, or take a plugin's. */
export function NoClassification({
  dialogs,
  sets,
}: {
  dialogs: ReturnType<typeof useAllocationDialogs>;
  sets: InstalledTaxonomySet[];
}) {
  const { t } = useLingui();
  return (
    <Empty
      title={t`No classification has been created`}
      action={
        <>
          <button type="button" className="btn" onClick={() => dialogs.openTaxonomy(null)}>
            <Trans>Create a classification</Trans>
          </button>
          <button
            type="button"
            className="btn btn--ghost"
            disabled={dialogs.busy !== null}
            onClick={() => dialogs.pickImport(null)}
          >
            <Trans>Import from CSV…</Trans>
          </button>
          {/* A ready tree a plugin brought. It goes through the same preview and the same
              commit as a file, so there is nothing extra to explain here. */}
          {sets.map((set) => (
            <button
              key={set.key}
              type="button"
              className="btn btn--ghost"
              disabled={dialogs.busy !== null}
              onClick={() => dialogs.importSet(set.key, null)}
            >
              {set.name}
            </button>
          ))}
        </>
      }
    >
      <Trans>
        A classification is a tree of categories and the weights of instruments in them. A global fund is 60 %
        United States and 40 % the rest of the world, which is why it is shares, not one category per
        instrument.
      </Trans>
    </Empty>
  );
}
