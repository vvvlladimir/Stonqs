import { useState } from "react";
import type { I18n } from "@lingui/core";
import { Trans, useLingui } from "@lingui/react/macro";
import {
  useAiModels,
  useAiProviders,
  useScope,
  useSecurities,
  useSettings,
  useTargets,
  useTaxonomies,
  useWatchlists,
} from "../../lib/queries";
import { useProviderName } from "../../lib/ai";
import { CheckField, Field, FormDialog, List, ListRow, Modal } from "../../components/ui";
import type { Dashboard as Board, Widget } from "../../lib/uiState";
import type { Security } from "../../lib/types";
import { periodLabel, usePeriodRanges } from "../../lib/periods";
import { METRICS } from "./widgets/metrics";

import { WIDGETS, type Field as WidgetField } from "./widgets";
import {
  MAX_BENCHMARKS,
  RATIO_TERMS,
  benchmarksOf,
  inflationOf,
  ratioTerms,
  lengthOf,
  modelOf,
  sourceFromKey,
  sourceKey,
  sourceOf,
} from "./widgets/model";
import { scopeLabel } from "../../components/domain/ScopePicker";
import { useAsOf } from "../../lib/asOf";

/** Widget catalog grouped for visual browsing. */
export function Palette({ onClose, onPick }: { onClose: () => void; onPick: (type: string) => void }) {
  const { t, i18n } = useLingui();
  // Group by the translated name: the catalog's sections are what the user reads.
  const groups = new Map<string, [string, (typeof WIDGETS)[string]][]>();
  for (const [type, def] of Object.entries(WIDGETS)) {
    const group = i18n._(def.group);
    groups.set(group, [...(groups.get(group) ?? []), [type, def]]);
  }

  return (
    <Modal title={t`Add widget`} onClose={onClose} wide>
      {[...groups].map(([group, list]) => (
        <div key={group}>
          <div className="group-label">{group}</div>
          <List variant="cards">
            {list.map(([type, def]) => {
              const Icon = def.icon;
              return (
                <ListRow
                  key={type}
                  box
                  top
                  wrap
                  lead={<Icon />}
                  title={i18n._(def.label)}
                  sub={i18n._(def.description)}
                  onClick={() => onPick(type)}
                />
              );
            })}
          </List>
        </div>
      ))}
    </Modal>
  );
}

/** Shared configuration dialog for all widget types. */
export function Config({
  widget,
  onClose,
  onSave,
}: {
  widget: Widget;
  onClose: () => void;
  onSave: (widget: Widget) => void;
}) {
  const { t, i18n } = useLingui();
  const def = WIDGETS[widget.type];
  const [draft, setDraft] = useState<Widget>({ ...widget, cfg: { ...widget.cfg } });
  const taxonomies = useTaxonomies();
  const securities = useSecurities();
  const targets = useTargets();
  const watchlists = useWatchlists();
  const scope = useScope();
  // The whole axis, user periods included — a widget may override the dashboard with any of them.
  const ranges = usePeriodRanges(useAsOf().date);

  const has = (field: WidgetField) => def.fields.includes(field);
  const set = (key: string, value: unknown) => setDraft({ ...draft, cfg: { ...draft.cfg, [key]: value } });
  const text = (key: string) => (typeof draft.cfg[key] === "string" ? (draft.cfg[key] as string) : "");

  return (
    <FormDialog title={i18n._(def.label)} onClose={onClose} onSubmit={() => onSave(draft)}>
      {has("title") && (
        <Field label={t`Title`}>
          <input
            type="text"
            value={text("title")}
            placeholder={def.titleOf ? def.titleOf(i18n, draft.cfg) : i18n._(def.label)}
            onChange={(e) => set("title", e.target.value)}
          />
        </Field>
      )}

      {has("source") && (
        <Field
          label={t`Data source`}
          hint={t`Left empty, the widget follows the picker like every screen does; naming one pins this tile to it.`}
          placeholder={t`Follow the picker`}
          options={(scope.data?.options ?? []).map((option) => ({
            value: sourceKey(option),
            label: scopeLabel(i18n, option),
          }))}
          value={sourceKey(sourceOf(draft))}
          onChange={(value) => set("source", sourceFromKey(value) ?? null)}
        />
      )}

      {has("metric") && (
        <Field
          label={t`Metric`}
          hint={metricTip(i18n, text("metric"))}
          options={Object.entries(METRICS).map(([key, metric]) => ({
            value: key,
            label: i18n._(metric.label),
          }))}
          value={text("metric")}
          onChange={(metric) => set("metric", metric)}
        />
      )}

      {has("ratio") && (
        <>
          <Field
            label={t`Top`}
            options={termOptions(i18n)}
            value={ratioTerms(draft.cfg).top}
            onChange={(top) => set("top", top)}
          />
          <Field
            label={t`Divided by`}
            options={termOptions(i18n)}
            value={ratioTerms(draft.cfg).bottom}
            onChange={(bottom) => set("bottom", bottom)}
          />
          <Field
            label={t`Shown as`}
            options={[
              { value: "percent", label: t`A percentage` },
              { value: "multiple", label: t`A multiple, like 2.5×` },
            ]}
            value={draft.cfg.as === "multiple" ? "multiple" : "percent"}
            onChange={(as) => set("as", as)}
          />
        </>
      )}

      {has("fire") && (
        <>
          <Field
            label={t`Spending to cover, a year`}
            hint={t`In the reporting currency. Everything else on this tile follows from it.`}
          >
            <input
              type="text"
              inputMode="decimal"
              value={text("spending")}
              placeholder={t`24000`}
              onChange={(e) => set("spending", e.target.value)}
            />
          </Field>
          <Field
            label={t`Withdrawal rate`}
            hint={t`The share of the capital taken out each year. 0.04 is the common rule of thumb and means the target is twenty-five years of spending.`}
          >
            <input
              type="text"
              inputMode="decimal"
              value={text("withdrawal")}
              placeholder="0.04"
              onChange={(e) => set("withdrawal", e.target.value)}
            />
          </Field>
          <Field
            label={t`Expected return, a year`}
            hint={t`An assumption, never this portfolio's measured return. Subtract inflation yourself to read the answer in today's money.`}
          >
            <input
              type="text"
              inputMode="decimal"
              value={text("return")}
              placeholder="0.05"
              onChange={(e) => set("return", e.target.value)}
            />
          </Field>
          <Field
            label={t`Paid in a month`}
            hint={t`Left empty, the tile uses what the active plans already add up to in a month.`}
          >
            <input
              type="text"
              inputMode="decimal"
              value={text("contribution")}
              placeholder={t`What the plans contribute`}
              onChange={(e) => set("contribution", e.target.value)}
            />
          </Field>
        </>
      )}

      {has("period") && (
        <Field
          label={t`Period`}
          hint={t`Overrides the dashboard period for this widget alone.`}
          placeholder={t`Same as the dashboard`}
          options={(ranges.data ?? []).map((r) => ({ value: r.id, label: periodLabel(i18n, r, false) }))}
          value={text("period")}
          onChange={(period) => set("period", period)}
        />
      )}

      {has("foot") && (
        <Field
          label={t`Line under the value`}
          hint={t`What the small line beneath the number says.`}
          placeholder={t`What the metric itself has to say`}
          options={[
            { value: "dates", label: t`The period's dates` },
            { value: "period", label: t`The period's name` },
            { value: "none", label: t`Nothing` },
          ]}
          value={text("foot")}
          onChange={(foot) => set("foot", foot)}
        />
      )}

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

      {has("prompt") && (
        <Field
          label={t`What this tile should say`}
          hint={t`Left empty, it writes what the period did and what drove it. Whatever you ask for, it still states only figures it was given.`}
        >
          <textarea
            rows={3}
            value={text("prompt")}
            placeholder={t`What moved the portfolio this period`}
            onChange={(e) => set("prompt", e.target.value)}
          />
        </Field>
      )}

      {has("length") && (
        <Field
          label={t`Answer length, tokens`}
          hint={t`A token is about three quarters of an English word, less in Russian. Left empty, the model decides how long to be.`}
        >
          <input
            type="number"
            min={50}
            max={4000}
            step={50}
            value={lengthOf(draft.cfg) ?? ""}
            placeholder={t`No limit`}
            // Only the ceiling is clamped while typing — a floor would turn the "3" of "300" into
            // 50. The host raises anything too small to be an answer.
            onChange={(e) => {
              const tokens = Math.round(Number(e.target.value));
              set("max_tokens", tokens > 0 ? Math.min(tokens, 4000) : null);
            }}
          />
        </Field>
      )}

      {has("refresh") && (
        <Field
          label={t`Write again on its own`}
          hint={t`Each rewrite is a paid request to the model, so this is off unless you ask for it. Off, the tile says when it is out of date and waits.`}
          placeholder={t`Only when I ask`}
          options={[
            { value: "daily", label: t`Once a day` },
            { value: "weekly", label: t`Once a week` },
            { value: "monthly", label: t`Once a month` },
          ]}
          value={text("refresh")}
          onChange={(refresh) => set("refresh", refresh)}
        />
      )}

      {has("model") && (
        <ModelFields
          widget={draft}
          onChange={(provider, model) => setDraft({ ...draft, cfg: { ...draft.cfg, provider, model } })}
        />
      )}

      {has("count") && (
        <Field label={t`Rows`}>
          <input
            type="number"
            min={3}
            max={20}
            value={Number(draft.cfg.count) || 6}
            onChange={(e) => set("count", Math.min(Math.max(Number(e.target.value) || 6, 3), 20))}
          />
        </Field>
      )}
    </FormDialog>
  );
}

/** Both sides of a ratio are picked from the same list; either may be a period figure. */
function termOptions(i18n: I18n) {
  return Object.entries(RATIO_TERMS).map(([key, term]) => ({ value: key, label: i18n._(term.label) }));
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

/**
 * Which provider and model write this tile. Its own component so the model list — a network call
 * — is asked for only by a widget that offers the choice. Switching provider clears the model:
 * an id from one catalogue names nothing in another.
 */
function ModelFields({
  widget,
  onChange,
}: {
  widget: Widget;
  onChange: (provider: string | null, model: string | null) => void;
}) {
  const { t } = useLingui();
  const name = useProviderName();
  const settings = useSettings();
  const providers = useAiProviders();
  const chosen = modelOf(widget);
  const listed = chosen.provider ?? settings.data?.ai_provider ?? null;
  const models = useAiModels(listed, true);
  // Every provider this build can talk to is listed, the user's own server once configured; one
  // without a key is shown but cannot be picked. The chosen one stays pickable whatever the
  // keychain says, so the select never shows a value it does not hold.
  const listedProviders = providers.data ?? [];
  const modelOptions = [...(models.data ?? [])];
  if (chosen.model && !modelOptions.includes(chosen.model)) modelOptions.unshift(chosen.model);

  return (
    <>
      <Field
        label={t`Provider`}
        hint={t`Left empty, the tile is written where a new chat begins.`}
        placeholder={t`Same as a new chat`}
        options={listedProviders.map((p) => {
          const usable = p.connected || p.id === chosen.provider;
          return {
            value: p.id,
            label: usable ? name(p.id, p.label) : t`${name(p.id, p.label)} — no key saved`,
            disabled: !usable,
          };
        })}
        value={chosen.provider ?? ""}
        onChange={(provider) => onChange(provider || null, null)}
      />
      <Field
        label={t`Model`}
        hint={t`Left empty, the newest model the provider offers.`}
        placeholder={models.isPending ? t`Loading the model list…` : t`The newest`}
        options={modelOptions.map((m) => ({ value: m, label: m }))}
        value={chosen.model ?? ""}
        // A model is stored with the provider it came from, so the default provider is written
        // down the moment a model of its is picked.
        onChange={(model) => onChange(model ? listed : chosen.provider, model || null)}
      />
    </>
  );
}

export function BoardName({
  action,
  current,
  onClose,
  onSave,
}: {
  action: "new" | "rename";
  current: string;
  onClose: () => void;
  onSave: (name: string) => void;
}) {
  const { t } = useLingui();
  const [name, setName] = useState(current || t`New dashboard`);
  return (
    <FormDialog
      title={action === "new" ? t`New dashboard` : t`Rename dashboard`}
      onClose={onClose}
      onSubmit={() => onSave(name.trim() || current || t`Dashboard`)}
      submitLabel={action === "new" ? t`Create` : t`Save`}
    >
      <Field
        label={t`Name`}
        hint={t`Dashboards are independent: each keeps its own widgets and their settings.`}
      >
        <input type="text" autoFocus value={name} onChange={(e) => setName(e.target.value)} />
      </Field>
    </FormDialog>
  );
}

/** The last board is never deleted: the screen would have nothing to show. */
export function DeleteBoard({
  board,
  boards,
  onClose,
  onDelete,
}: {
  board: Board;
  boards: Board[];
  onClose: () => void;
  onDelete: () => void;
}) {
  const { t } = useLingui();
  const last = boards.length === 1;
  return (
    <Modal
      title={last ? t`Cannot delete` : t`Delete dashboard`}
      onClose={onClose}
      foot={
        <>
          <button type="button" className="btn btn--ghost" onClick={onClose}>
            {last ? t`Close` : t`Cancel`}
          </button>
          {!last && (
            <button type="button" className="btn btn--danger" onClick={onDelete}>
              <Trans>Delete</Trans>
            </button>
          )}
        </>
      }
    >
      {last ? (
        <p className="muted">
          <Trans>The last dashboard stays. Rename it, or clear its layout.</Trans>
        </p>
      ) : (
        <p className="muted">
          <Trans>
            Delete "{board.name}" and its {board.widgets.length} widgets? Accounts and transactions are
            untouched — a dashboard stores only a layout.
          </Trans>
        </p>
      )}
    </Modal>
  );
}

/** The metric tip, resolved only when the chosen metric exists. */
function metricTip(i18n: I18n, key: string): string | undefined {
  const metric = METRICS[key];
  return metric ? i18n._(metric.tip) : undefined;
}
