import { useState, type ReactNode } from "react";
import { useMutation } from "@tanstack/react-query";
import { api } from "../../../lib/api";
import {
  affects,
  keys,
  useInvalidate,
  useMarketCustom,
  useMarketSources,
  useSecurities,
  useSettings,
} from "../../../lib/queries";
import { SourcesActions, SourcesBody, type Finish } from "./SourcesPicker";
import { sourceNeeds } from "./model";
import type { MarketSourceRow } from "../../../lib/types";

interface Options {
  /** The paragraph saying what a source is; off where the frame already says it. */
  intro?: boolean;
  /** Runs once the answer is written, before `onDone`, whichever way the question was left. */
  after?: () => Promise<unknown>;
  onDone: () => void;
}

/** The sources question (ADR-0076), framed by its caller: a dialog, or a step of onboarding. */
export function useSourcesSetup({ intro = true, after, onDone }: Options): {
  body: ReactNode;
  actions: ReactNode;
} {
  const invalidate = useInvalidate();
  const sources = useMarketSources();
  const custom = useMarketCustom();
  const settings = useSettings();
  const securities = useSecurities();
  const rows = sources.data ?? [];
  const mine = custom.data ?? [];

  // A switch is itself the answer (ADR-0076), so the settings change with the rows.
  const refresh = () =>
    invalidate(keys.settings(), keys.marketSources(), keys.marketCustom(), keys.quoteProviders());
  const switchOne = useMutation({
    mutationFn: ({ id, on }: { id: string; on: boolean }) => api.marketSourceSwitch(id, on),
    onSuccess: refresh,
  });

  // Everything that can answer without one: a source whose key is required and missing would be
  // switched on into the state that reads as broken.
  const available = rows.filter((row) => row.key !== "required" || row.has_key);
  const needs = sourceNeeds(rows, mine, settings.data?.market_sources ?? {});

  // An instrument created while no source was on is priced by hand and no refresh would ever
  // notice it, so the first source that arrives is offered to them all at once.
  const orphans = (securities.data ?? []).filter((s) => s.data_source === null).length;
  const firstQuotes = rows.find((row) => row.wanted && quotes(row) && row.key !== "required")?.id;
  const [adopt, setAdopt] = useState(true);

  // `all` is one press for the whole question; its switches are written one at a time, which is
  // what the picker itself does.
  const finish = useMutation({
    mutationFn: async (how: Finish) => {
      if (how === "all") {
        for (const row of available.filter((row) => !row.wanted)) await api.marketSourceSwitch(row.id, true);
      }
      if (how !== "later") {
        await api.marketSourcesConfirm();
        if (adopt && orphans > 0 && firstQuotes) await api.securitiesAdoptSource(firstQuotes);
      }
      await after?.();
    },
    onSuccess: () => {
      invalidate(keys.settings(), keys.marketSources(), keys.quoteProviders(), ...affects.securities);
      onDone();
    },
  });

  const busy = finish.isPending || switchOne.isPending;
  const body = (
    <SourcesBody
      intro={intro}
      rows={rows}
      mine={mine}
      needs={needs}
      busy={busy}
      onSwitch={(id, on) => switchOne.mutate({ id, on })}
      onRefresh={refresh}
      adoption={orphans > 0 && firstQuotes ? { orphans, source: firstQuotes, adopt, setAdopt } : null}
    />
  );
  const actions = (
    <SourcesActions
      configured={settings.data?.sources_configured ?? false}
      needs={needs}
      busy={busy}
      allOn={available.length > 0 && available.every((row) => row.wanted)}
      pending={finish.isPending ? finish.variables : null}
      error={finish.error ?? switchOne.error}
      onFinish={(how) => finish.mutate(how)}
    />
  );
  return { body, actions };
}

/** Whether the source can answer with prices at all — a rate or venue source cannot. */
function quotes(row: MarketSourceRow): boolean {
  return row.capabilities.includes("quotes");
}
