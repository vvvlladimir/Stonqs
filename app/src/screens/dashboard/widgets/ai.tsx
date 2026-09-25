import { useCallback, useEffect, useRef, useState } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { SparkleIcon } from "@phosphor-icons/react";
import { api, ApiError } from "../../../lib/api";
import { BRIEF_READINGS } from "../../../lib/ai";
import { useChangedSince } from "../../../lib/freshness";
import { formatDateTime } from "../../../lib/format";
import { pickRange, usePeriodRanges } from "../../../lib/periods";
import { useSettings } from "../../../lib/queries";
import { useUiState } from "../../../lib/uiState";
import { ErrorText, Markdown, useUiErrorText } from "../../../components/ui";
import { TOOL_LABELS } from "../../../components/domain/aiToolLabels";
import type { AiEvent, UiError } from "../../../lib/types";
import { lengthOf, modelOf, periodOf, refreshEvery, sourceOf, type WidgetProps } from "./model";

/**
 * A written summary of the period, generated on a button and never on its own: a tile that
 * regenerated itself when the data moved would be spending the user's money quietly. When the
 * figures under it do change, the brief says it is out of date and waits (ADR-0039).
 *
 * The tile reads a fixed set — `BRIEF_READINGS`, named in the host — and lists it before the
 * first generation, so pressing the button is the permission rather than something claimed on
 * the user's behalf.
 *
 * A rewriting interval is offered and is off unless chosen: every rewrite is a paid request, so
 * the tile spends nothing until the user says how often it may (ADR-0040).
 */
export function BriefWidget({ widget, date, period }: WidgetProps) {
  const { t, i18n } = useLingui();
  const { ui, save } = useUiState();
  const settings = useSettings();
  const ranges = usePeriodRanges(date);
  const range = pickRange(ranges.data, periodOf(widget, period));

  const brief = ui.ai_briefs[widget.id];
  const [live, setLive] = useState<string | null>(null);
  const [error, setError] = useState<UiError | null>(null);
  // Quotes and transactions are what a summary is about; a renamed account does not stale it.
  const changed = useChangedSince(brief?.at, ["quotes", "transactions", "portfolio"]);
  const elsewhere =
    brief !== undefined && range !== undefined && (brief.from !== range.from || brief.to !== range.to);

  // What the timer further down needs, refreshed after each render rather than read during one:
  // the interval outlives the render that started it and must not hold a stale brief or a stale
  // generate. `failed` stops an automatic rewrite that errored from retrying every hour.
  const latest = useRef<{ generate: () => Promise<void>; at?: string; busy: boolean } | null>(null);
  const failed = useRef(false);

  const generate = useCallback(async () => {
    if (!range) return;
    setError(null);
    // A press is also how a tile whose automatic rewrite failed gets another chance: nothing
    // retries on a timer, so a rejected key is asked about once, not once an hour forever.
    failed.current = false;
    setLive("");
    let text = "";
    try {
      await api.aiBrief(
        range.from,
        range.to,
        sourceOf(widget),
        typeof widget.cfg.prompt === "string" && widget.cfg.prompt.trim() !== "" ? widget.cfg.prompt : null,
        // The app's language, not the data's: the tile sits in the interface and reads like it.
        i18n.locale,
        modelOf(widget),
        lengthOf(widget.cfg),
        (event: AiEvent) => {
          switch (event.type) {
            case "text":
              text += event.text;
              setLive(text);
              break;
            case "done":
              // Written once, at the end: a half-streamed paragraph is not worth keeping, and
              // the board's state is saved to the host on every change.
              void save((ui) => ({
                ...ui,
                ai_briefs: {
                  ...ui.ai_briefs,
                  [widget.id]: { text, at: new Date().toISOString(), from: range.from, to: range.to },
                },
              }));
              setLive(null);
              break;
            case "error":
              setError(event.error);
              failed.current = true;
              setLive(null);
              break;
          }
        },
      );
    } catch (e) {
      // The command refused before the call started (tile off, no key saved): keep its code, the
      // way the panel does — the sentence for it is written from the code, never from the host.
      setError(e instanceof ApiError ? e.detail : { code: "internal", message: String(e) });
      failed.current = true;
      setLive(null);
    }
  }, [range, widget, i18n.locale, save]);

  useEffect(() => {
    latest.current = { generate, at: brief?.at, busy: live !== null };
  });

  const every = refreshEvery(widget.cfg);
  const enabled = settings.data?.ai_enabled ?? false;
  useEffect(() => {
    if (!every || !enabled || !range) return;
    const due = () => {
      const now = latest.current;
      if (!now || now.busy || failed.current) return;
      const age = now.at ? Date.now() - Date.parse(now.at) : Number.POSITIVE_INFINITY;
      if (age >= every) void now.generate();
    };
    due();
    // Checked hourly rather than slept for the whole interval: a laptop that was closed for a
    // week must notice on waking, not an interval later.
    const timer = setInterval(due, Math.min(every, 3600 * 1000));
    return () => clearInterval(timer);
  }, [every, enabled, range]);

  if (settings.data && !settings.data.ai_enabled)
    return (
      <p className="muted">
        <Trans>Turn the assistant on in Settings to use this tile.</Trans>
      </p>
    );

  return (
    <div className="brief">
      {live !== null ? (
        <Markdown>{live || t`Reading the portfolio…`}</Markdown>
      ) : brief ? (
        <Markdown>{brief.text}</Markdown>
      ) : (
        <p className="muted">
          <Trans>This summary reads:</Trans>{" "}
          {BRIEF_READINGS.map((tool) => (TOOL_LABELS[tool] ? i18n._(TOOL_LABELS[tool]) : tool)).join(", ")}.
        </p>
      )}

      {error && <BriefError error={error} />}

      <div className="brief__foot">
        {brief && (
          <span className="muted">
            {elsewhere ? (
              <Trans>Written about another period, {formatDateTime(brief.at)}</Trans>
            ) : changed ? (
              <Trans>Out of date since the figures moved, {formatDateTime(brief.at)}</Trans>
            ) : (
              <Trans>Written {formatDateTime(brief.at)}</Trans>
            )}
          </span>
        )}
        <span className="spacer" />
        <button
          type="button"
          className="btn btn--ghost btn--sm"
          disabled={live !== null || !range}
          onClick={() => void generate()}
        >
          <SparkleIcon /> {brief ? <Trans>Write again</Trans> : <Trans>Write a summary</Trans>}
        </button>
      </div>
    </div>
  );
}

/** The same red line the panel uses: a code from the host, never a raw message. */
function BriefError({ error }: { error: UiError }) {
  return <ErrorText>{useUiErrorText(error)}</ErrorText>;
}
