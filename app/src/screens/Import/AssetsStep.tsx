import { plural } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { useEffect, useRef, useState } from "react";
import { api } from "../../lib/api";
import { Banner, Buttons, ErrorText, List, Panel } from "../../components/ui";
import { MapRow } from "./MapRow";
import type { ImportMapping, ImportPreviewData, SecurityDraft } from "../../lib/types";
import { AssetRow } from "./AssetRow";
import { normalizeAlias } from "./labels";

/** Resolve broker identifiers to instruments without changing the preview input. */
export function AssetsStep({
  preview,
  mapping,
  onChange,
}: {
  preview: ImportPreviewData;
  mapping: ImportMapping;
  onChange: (m: ImportMapping) => void;
}) {
  const { t } = useLingui();
  const [progress, setProgress] = useState<string | null>(null);
  const [failed, setFailed] = useState<string[]>([]);
  // The search runs once per visit to this step, not once per render: it is network, and the
  // mapping it writes re-renders the step that started it.
  const searched = useRef(false);

  const needed = preview.symbols.filter((s) => s.required);
  const known = needed.filter((s) => s.security_id);
  const planned = needed.filter((s) => !s.security_id && s.planned);
  const unknown = needed.filter((s) => !s.security_id && !s.planned);
  const cashOnly = preview.symbols.filter((s) => !s.required);

  const setAlias = (from: string, to: string) => {
    const symbol_aliases = { ...mapping.symbol_aliases };
    const key = normalizeAlias(from);
    if (to) symbol_aliases[key] = to.toUpperCase();
    else delete symbol_aliases[key];
    onChange({ ...mapping, symbol_aliases });
  };

  const setPlanned = (value: string, draft: SecurityDraft | null) => {
    const new_securities = { ...mapping.new_securities };
    const key = normalizeAlias(value);
    if (draft) new_securities[key] = draft;
    else delete new_securities[key];
    onChange({ ...mapping, new_securities });
  };

  /** Resolve unknown instruments sequentially to avoid provider rate limits. */
  const resolveAll = async () => {
    const new_securities = { ...mapping.new_securities };
    const missed: string[] = [];
    for (const [i, symbol] of unknown.entries()) {
      setProgress(t`${symbol.value} · ${i + 1} of ${unknown.length}`);
      try {
        const draft = await api.importResolveSymbol(
          symbol.resolved,
          symbol.isin,
          symbol.file_name,
          symbol.currency,
        );
        if (draft) new_securities[normalizeAlias(symbol.value)] = draft;
        else missed.push(symbol.value);
      } catch {
        missed.push(symbol.value);
      }
    }
    setProgress(null);
    setFailed(missed);
    onChange({ ...mapping, new_securities });
  };

  // An instrument nobody identified enters the portfolio under the export's own code and stays
  // without prices, which is only discovered days later on a chart that has none. The step
  // therefore does its own work on arrival and leaves the corrections to the user.
  useEffect(() => {
    if (searched.current || unknown.length === 0) return;
    searched.current = true;
    void resolveAll();
    // Started once on arrival; `resolveAll` closes over the mapping it was mounted with.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <>
      <Banner tone={unknown.length === 0 ? "info" : "warn"}>
        {progress
          ? t`Identifying instruments: ${progress}`
          : unknown.length === 0
            ? t`Every instrument is identified. Their quotes will arrive automatically.`
            : t`${plural(unknown.length, { one: "# instrument", other: "# instruments" })} not identified. Such an instrument enters the portfolio under the export's code and stays without quotes.`}
      </Banner>

      {unknown.length > 0 && (
        <Panel
          title={t`Not identified`}
          note={String(unknown.length)}
          tools={
            <Buttons>
              <button className="btn btn--sm" disabled={progress !== null} onClick={resolveAll}>
                {progress ? t`Searching ${progress}…` : t`Search again`}
              </button>
            </Buttons>
          }
        >
          <p className="muted">
            <Trans>
              The search runs by itself when the step opens: by ISIN first, then by the column value and the
              name from the file, and finally over the exchanges the instrument trades on, taking the first
              that actually has prices. What it finds can be corrected or replaced with another listing.
            </Trans>
          </p>
          {failed.length > 0 && (
            <ErrorText>
              <Trans>
                Not found: {failed.join(", ")}. They can be linked to an instrument already in the database,
                or left as they are — prices are then entered by hand.
              </Trans>
            </ErrorText>
          )}
          <List>
            {unknown.map((s) => (
              <AssetRow
                key={s.value}
                symbol={s}
                aliases={mapping.symbol_aliases}
                onAlias={setAlias}
                onPlan={setPlanned}
              />
            ))}
          </List>
        </Panel>
      )}

      {planned.length > 0 && (
        <Panel title={t`Will be created`} note={String(planned.length)}>
          <List>
            {planned.map((s) => (
              <AssetRow
                key={s.value}
                symbol={s}
                aliases={mapping.symbol_aliases}
                onAlias={setAlias}
                onPlan={setPlanned}
              />
            ))}
          </List>
        </Panel>
      )}

      {known.length > 0 && (
        <Panel title={t`Already in the portfolio`} note={String(known.length)}>
          <List>
            {known.map((s) => (
              <AssetRow
                key={s.value}
                symbol={s}
                aliases={mapping.symbol_aliases}
                onAlias={setAlias}
                onPlan={setPlanned}
              />
            ))}
          </List>
        </Panel>
      )}

      {cashOnly.length > 0 && (
        <Panel title={t`Symbols on cash transactions`} note={String(cashOnly.length)}>
          <p className="muted">
            <Trans>
              These values appear only on rows that need no instrument: a transfer, a fee. No instrument has
              to be created for them.
            </Trans>
          </p>
          <List>
            {cashOnly.map((s) => (
              <MapRow key={s.value} value={s.value} occurrences={s.count} />
            ))}
          </List>
        </Panel>
      )}
    </>
  );
}
