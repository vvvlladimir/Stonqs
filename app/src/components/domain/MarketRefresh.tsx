import { useEffect, useState } from "react";
import { useMutation, useQuery } from "@tanstack/react-query";
import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { api, onMarketProgress } from "../../lib/api";
import { affects, keys, useInvalidate } from "../../lib/queries";
import { formatDateTime } from "../../lib/format";
import { Buttons, ErrorText, ListRow } from "../ui";
import type { JobFailure, Progress, RefreshMode, RefreshStatus } from "../../lib/types";

/** Combines initial refresh status with live progress events. */
export function useRefreshStatus() {
  const invalidate = useInvalidate();
  const initial = useQuery({ queryKey: keys.refreshStatus(), queryFn: api.refreshStatus });
  const [live, setLive] = useState<Progress | null>(null);

  useEffect(() => {
    const unlisten = onMarketProgress((progress) => {
      setLive(progress);
      if (progress.event === "finished") {
        invalidate(...affects.quotes);
      }
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [invalidate]);

  const status: RefreshStatus | undefined = initial.data;
  const running = live === null ? (status?.running ?? false) : live.event !== "finished";

  return { status, live, running };
}

/** The wording of a refresh failure: the host sends a code and a subject, never a sentence. */
function headline(i18n: I18n, failure: JobFailure): string {
  switch (failure.code) {
    case "database":
      return i18n._(msg`Database`);
    case "securities":
      return i18n._(msg`Instrument list`);
    case "currencies":
      return i18n._(msg`Currency list`);
    case "quote":
      return failure.subject;
    case "rate":
      return failure.subject;
    case "unidentified":
      return i18n._(msg`Not identified: ${failure.subject}`);
    case "internal":
      return i18n._(msg`Refresh`);
  }
}

/** What the user should do about a failure. `null` when only the host's own detail can say. */
function remedy(i18n: I18n, failure: JobFailure): string | null {
  const { code, cause, subject } = failure;
  const source = failure.source ?? i18n._(msg`the data source`);
  if (code === "unidentified") {
    return i18n._(
      msg`${subject}: the ticker field holds an ISIN, which has no prices. Open Instruments and choose Identify or Venues… for it.`,
    );
  }
  if (code === "internal") return i18n._(msg`The refresh stopped unexpectedly. Try again.`);
  if (cause === "storage" || code === "database" || code === "securities" || code === "currencies") {
    return i18n._(msg`The local database could not be read. Restart the app and try again.`);
  }
  if (cause === "unauthorized") {
    return i18n._(
      msg`${subject}: ${source} refused the API key. Open Settings, Market data, and replace the key or turn the source off.`,
    );
  }
  if (cause === "unreachable") {
    return i18n._(
      msg`${subject}: ${source} did not answer (no connection, timeout, throttling or its daily limit). Try again later.`,
    );
  }
  if (code === "quote" && (cause === "no_data" || cause === "rejected")) {
    return i18n._(
      msg`${subject}: ${source} has no prices for this ticker — it is misspelled, delisted or from an unsupported venue. Open Instruments and fix the ticker, or pick another listing with Venues….`,
    );
  }
  if (code === "rate" && (cause === "no_data" || cause === "rejected")) {
    return i18n._(
      msg`${subject}: no data source publishes a rate for this currency. Check the currency of the instruments and accounts that use it.`,
    );
  }
  return null;
}

/** One line per failure: what to fix, or the headline with the host's detail when nothing more is known. */
function failureLine(i18n: I18n, failure: JobFailure): string {
  return remedy(i18n, failure) ?? `${headline(i18n, failure)}: ${failure.detail}`;
}

/** Failures listed in the chip's tooltip before the rest are only counted. */
const TIP_FAILURES = 4;

export function MarketRefresh() {
  const { i18n } = useLingui();
  const { status, live, running } = useRefreshStatus();
  const refresh = useMutation({ mutationFn: (mode: RefreshMode) => api.marketRefresh(mode) });
  const cancel = useMutation({ mutationFn: api.marketRefreshCancel });

  const failures = live?.event === "finished" ? live.failed : (status?.failures ?? []);
  const cancelled = live?.event === "finished" ? live.cancelled : (status?.cancelled ?? false);

  return (
    <ListRow
      box
      top
      wrap
      title={<Trans>Quotes and exchange rates</Trans>}
      sub={
        <Trans>
          Yahoo and the ECB. Startup fetches the tail only — from the last known date; a full download pulls
          five years of history.
        </Trans>
      }
      end={
        <Buttons>
          {running ? (
            // Cancellation preserves already fetched data and stops the queue.
            <button className="btn btn--ghost" onClick={() => cancel.mutate()}>
              <Trans>Stop</Trans>
            </button>
          ) : (
            <>
              <button className="btn" onClick={() => refresh.mutate("catch_up")}>
                <Trans>Refresh</Trans>
              </button>
              <button className="btn btn--ghost" onClick={() => refresh.mutate("full")}>
                <Trans>Download history</Trans>
              </button>
            </>
          )}
        </Buttons>
      }
      foot={
        <>
          {running && live?.event === "item" && (
            <span>
              <Trans>
                {live.label} · {live.done} of {live.total} · {live.fetched} downloaded
              </Trans>
            </span>
          )}
          {running && live?.event === "started" && (
            <span>
              <Trans>{live.total} to refresh in total</Trans>
            </span>
          )}
          {running && live === null && status?.running && (
            <span>
              <Trans>
                A refresh is already running: {status.label ?? "…"} · {status.done} of {status.total}
              </Trans>
            </span>
          )}
          {!running && live?.event === "finished" && (
            <span>
              {live.cancelled ? <Trans>Stopped</Trans> : <Trans>Done</Trans>}{" "}
              <Trans>{live.fetched} records downloaded.</Trans>
            </span>
          )}
          {!running && live === null && cancelled && (
            <span>
              <Trans>The previous refresh was stopped — the list was not walked to the end.</Trans>
            </span>
          )}
          {!running && live === null && status?.last_finished && (
            <span>
              <Trans>Last refresh: {formatDateTime(status.last_finished)}</Trans>
            </span>
          )}
          {failures.map((f, i) => (
            <ErrorText as="div" key={`${f.code}-${f.subject}-${i}`}>
              {failureLine(i18n, f)}
            </ErrorText>
          ))}
        </>
      }
    />
  );
}

/** Persistent refresh status chip with idle, loading, and error states. */
export function SyncChip() {
  const { t, i18n } = useLingui();
  const { status, live, running } = useRefreshStatus();
  const refresh = useMutation({ mutationFn: (mode: RefreshMode) => api.marketRefresh(mode) });

  const failures = running ? [] : live?.event === "finished" ? live.failed : (status?.failures ?? []);
  const failed = failures.length > 0;
  const rest = failures.length - TIP_FAILURES;
  const tip = running
    ? t`Quotes and rates are refreshing`
    : failed
      ? [
          ...failures.slice(0, TIP_FAILURES).map((f) => failureLine(i18n, f)),
          ...(rest > 0 ? [t`…and ${rest} more in Settings → Market data.`] : []),
          t`Click to try again.`,
        ].join("\n\n")
      : t`Refresh quotes and rates`;
  const state = running ? "loading" : failed ? "error" : "idle";
  const label =
    running && live?.event === "item"
      ? `${live.done}/${live.total}`
      : running
        ? "…"
        : failed
          ? t`Errors`
          : t`Data`;

  return (
    <button
      type="button"
      className="iconbtn chip-sync"
      data-state={state}
      data-tip={tip}
      disabled={running}
      onClick={() => refresh.mutate("catch_up")}
    >
      <span className="dot" />
      <span className="chip-sync__text">{label}</span>
    </button>
  );
}
