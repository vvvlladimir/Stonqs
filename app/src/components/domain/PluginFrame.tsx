import { useEffect, useMemo, useRef, useState } from "react";
import { useMutation } from "@tanstack/react-query";
import { useLingui } from "@lingui/react/macro";
import { api, type Source } from "../../lib/api";
import { useLanguage } from "../../lib/i18n";
import { keys, useInvalidate, usePluginReads, usePluginState } from "../../lib/queries";
import {
  BRIDGE_API,
  complete,
  isFrameMessage,
  periodOfRange,
  projectPerformance,
  projectPositions,
  projectTransactions,
  projectValuation,
  renderMessage,
  useThemeTokens,
  type BridgeData,
} from "../../lib/pluginBridge";
import { ErrorText, QueryError } from "../ui";
import { useErrorText } from "../ui/Async";
import type { DateString, PeriodRange, WidgetRead } from "../../lib/types";

/** How long a frame may take to say it is there before it is called broken. */
const START_MS = 10_000;

export interface PluginPage {
  kind: "widget" | "screen";
  /** `<plugin id>/<page id>`. */
  key: string;
  plugin: string;
  name: string;
  reads: WidgetRead[];
  /** Keeps one document in the profile (a screen only, ADR-0084). */
  storage: boolean;
}

interface Failure {
  code: "threw" | "no_render" | "no_start";
  detail: string;
}

/**
 * A plugin's page in a frame with no origin, fed the reads it declared and nothing else
 * (ADR-0083/0084). What surrounds it — a tile, a screen's header — is the caller's.
 */
export function PluginFrame({
  page,
  date,
  range,
  source,
}: {
  page: PluginPage;
  date: DateString;
  /** The period its period reads are answered for; absent for a page that reads none. */
  range?: PeriodRange;
  source?: Source;
}) {
  const { t } = useLingui();
  const invalidate = useInvalidate();
  const frame = useRef<HTMLIFrameElement | null>(null);
  const [ready, setReady] = useState(false);
  const [failure, setFailure] = useState<Failure | null>(null);
  const reads = usePluginReads(page.reads, date, range, source);
  const state = usePluginState(page.storage ? page.plugin : null);
  const theme = useThemeTokens();
  const { locale } = useLanguage();

  const keep = useMutation({
    mutationFn: (document: unknown) => api.pluginStateSave(page.plugin, document),
    onSuccess: () => invalidate(keys.pluginState(page.plugin)),
  });
  // `mutate` is stable across renders, so the listener below subscribes once.
  const save = keep.mutate;

  // Only this page's own frame is listened to: every plugin frame posts to the same window.
  useEffect(() => {
    const onMessage = (event: MessageEvent) => {
      if (event.source !== frame.current?.contentWindow || !isFrameMessage(event.data)) return;
      const message = event.data;
      if (message.type === "ready") setReady(true);
      // A page that declared no storage has nowhere to keep anything; the host would refuse too.
      else if (message.type === "save") {
        if (page.storage) save(message.state);
      } else setFailure({ code: message.code, detail: message.detail });
    };
    window.addEventListener("message", onMessage);
    return () => window.removeEventListener("message", onMessage);
  }, [page.storage, save]);

  useEffect(() => {
    if (ready) return;
    const timer = window.setTimeout(() => setFailure({ code: "no_start", detail: "" }), START_MS);
    return () => window.clearTimeout(timer);
  }, [ready]);

  const data = useMemo<BridgeData>(
    () => ({
      valuation: reads.valuation.data ? projectValuation(reads.valuation.data) : undefined,
      positions: reads.positions.data ? projectPositions(reads.positions.data) : undefined,
      performance: reads.performance.data ? projectPerformance(reads.performance.data) : undefined,
      transactions: reads.transactions.data ? projectTransactions(reads.transactions.data) : undefined,
      state: page.storage ? state.data : undefined,
    }),
    [
      reads.valuation.data,
      reads.positions.data,
      reads.performance.data,
      reads.transactions.data,
      page.storage,
      state.data,
    ],
  );
  const base =
    data.valuation?.base_currency ??
    data.positions?.base_currency ??
    data.performance?.base_currency ??
    data.transactions?.base_currency ??
    null;

  useEffect(() => {
    const target = frame.current?.contentWindow;
    if (!ready || !target || !complete(page.reads, data, page.storage)) return;
    const context = {
      api: BRIDGE_API,
      date,
      period: periodOfRange(range),
      base_currency: base,
      locale,
      theme,
    };
    // The frame has no origin to address, so `*`: what is posted is only what it declared it reads.
    target.postMessage(renderMessage(context, data), "*");
  }, [ready, page.reads, page.storage, data, date, range, base, locale, theme]);

  const queryError = [reads.valuation, reads.positions, reads.performance, reads.transactions, state].find(
    (q) => q.isError,
  )?.error;
  if (queryError) return <QueryError error={queryError} />;
  if (failure) {
    const detail = failure.detail;
    return (
      <ErrorText>
        {failure.code === "no_start"
          ? t`This plugin's page did not start.`
          : failure.code === "no_render"
            ? t`This plugin's page never drew anything.`
            : t`This plugin's page failed: ${detail}`}
      </ErrorText>
    );
  }
  return (
    <>
      {keep.isError && <SaveError error={keep.error} />}
      <iframe
        ref={frame}
        className="plugin-frame"
        data-kind={page.kind}
        src={api.pluginPageUrl(page.kind, page.key)}
        // Scripts and nothing else: no origin, no storage, no popups, no forms, no navigation.
        sandbox="allow-scripts"
        title={page.name}
      />
    </>
  );
}

function SaveError({ error }: { error: unknown }) {
  return <ErrorText>{useErrorText(error)}</ErrorText>;
}
