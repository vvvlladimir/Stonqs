import { useLingui } from "@lingui/react/macro";
import { CheckField, Field } from "../../../../components/ui";
import { useAsOf } from "../../../../lib/asOf";
import {
  useGoals,
  useLimits,
  useSecurities,
  useTargets,
  useTaxonomies,
  useWatchlists,
} from "../../../../lib/queries";
import type { Security } from "../../../../lib/types";
import { MAX_BENCHMARKS, benchmarksOf, inflationOf } from "../../widgets/model";
import type { FieldsCtx } from "./context";

/** What the tile is about: a tree, benchmarks, a target, a list, a goal or a limit. */
export function SubjectFields({ ctx }: { ctx: FieldsCtx }) {
  const { t } = useLingui();
  const { draft, setDraft, has, tracks, set, text } = ctx;
  const taxonomies = useTaxonomies();
  const securities = useSecurities();
  const targets = useTargets();
  const watchlists = useWatchlists();
  const goals = useGoals(useAsOf().date);
  const limits = useLimits(useAsOf().date);
  return (
    <>
      {has("taxonomy") && (
        <Field
          label={t`Breakdown`}
          placeholder={t`By instrument`}
          options={(taxonomies.data ?? []).map((tree) => ({ value: tree.id, label: tree.name }))}
          value={text("taxonomy")}
          onChange={(taxonomy) => set("taxonomy", taxonomy)}
        />
      )}

      {has("benchmark") && (
        <BenchmarkFields
          chosen={benchmarksOf(draft.cfg)}
          securities={securities.data ?? []}
          // The list replaces the single id older boards stored, so that one is dropped here.
          onChange={(benchmarks) =>
            setDraft({ ...draft, cfg: { ...draft.cfg, benchmarks, benchmark: undefined } })
          }
        />
      )}

      {has("inflation") && (
        <CheckField
          label={t`Draw inflation`}
          hint={t`The cost of money in the portfolio's price-index region, set in Settings. The line stops at the last month published.`}
          checked={inflationOf(draft.cfg)}
          onChange={(inflation: boolean) => set("inflation", inflation)}
        />
      )}

      {has("target") && (
        <Field
          label={t`Target`}
          hint={t`Left empty, the widget follows the first target — a new one does not open blank.`}
          placeholder={t`The first target`}
          options={(targets.data ?? []).map((target) => ({ value: target.id, label: target.name }))}
          value={text("target")}
          onChange={(target) => set("target", target)}
        />
      )}

      {has("watchlist") && (
        <Field
          label={t`Watchlist`}
          hint={t`Left empty, the widget shows the first list.`}
          placeholder={t`The first list`}
          options={(watchlists.data ?? []).map((list) => ({ value: list.id, label: list.name }))}
          value={text("watchlist")}
          onChange={(watchlist) => set("watchlist", watchlist)}
        />
      )}

      {tracks("goal") && (
        <Field
          label={t`Goal`}
          hint={t`Left empty, the widget shows the first goal.`}
          placeholder={t`The first goal`}
          options={(goals.data ?? []).map((row) => ({ value: row.goal.id, label: row.goal.name }))}
          value={text("goal")}
          onChange={(goal) => set("goal", goal)}
        />
      )}

      {tracks("limit") && (
        <Field
          label={t`Limit`}
          hint={t`Left empty, the widget shows the first limit.`}
          placeholder={t`The first limit`}
          options={(limits.data ?? []).map((usage) => ({ value: usage.limit_id, label: usage.name }))}
          value={text("limit")}
          onChange={(limit) => set("limit", limit)}
        />
      )}
    </>
  );
}

/**
 * The instruments a benchmark chart compares against: one select per line, and a blank one after
 * them while there is room for another. Emptying a select removes that line.
 */
function BenchmarkFields({
  chosen,
  securities,
  onChange,
}: {
  chosen: string[];
  securities: Security[];
  onChange: (benchmarks: string[]) => void;
}) {
  const { t } = useLingui();
  const slots = chosen.length < MAX_BENCHMARKS ? [...chosen, ""] : chosen;
  const pick = (index: number, id: string) => {
    const next = [...chosen];
    if (id === "") next.splice(index, 1);
    else next[index] = id;
    onChange(next);
  };

  return (
    <>
      {slots.map((current, index) => (
        <Field
          key={`${index}:${current}`}
          label={slots.length > 1 ? t`Benchmark ${index + 1}` : t`Benchmark`}
          hint={
            index === slots.length - 1
              ? t`Each line is computed on the portfolio series' dates, so the lines do not drift apart on another exchange's closed days.`
              : undefined
          }
          placeholder={current === "" && index > 0 ? t`Add a benchmark` : t`No benchmark`}
          // An instrument already drawn by another line is not offered twice.
          options={securities
            .filter((sec) => sec.id === current || !chosen.includes(sec.id))
            .map((sec) => ({ value: sec.id, label: `${sec.symbol} — ${sec.name}` }))}
          value={current}
          onChange={(id) => pick(index, id)}
        />
      ))}
    </>
  );
}
