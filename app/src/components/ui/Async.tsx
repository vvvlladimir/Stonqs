import type { ReactNode } from "react";
import type { UseQueryResult } from "@tanstack/react-query";
import { msg } from "@lingui/core/macro";
import { Trans, useLingui } from "@lingui/react/macro";
import { ApiError } from "../../lib/api";
import type { UiError } from "../../lib/types";
import { Banner } from "./Banner";

/** The single wording for "the answer is not here yet" — never spelled out anywhere else. */
export function Pending() {
  return (
    <p className="muted">
      <Trans>Calculating…</Trans>
    </p>
  );
}

/**
 * A host failure as one line: the headline comes from the error's code, so it is in the
 * user's language, and the host's English detail follows it. Takes the payload rather than a
 * thrown error, because not every failure arrives as one — a streamed turn hands its `UiError`
 * over the channel (`lib/ai.ts`), and it deserves the same wording as a rejected command.
 */
export function useUiErrorText(detail: UiError): string {
  const { i18n } = useLingui();
  switch (detail.code) {
    case "storage":
      return i18n._(msg`Database error: ${detail.message}`);
    case "network":
      return i18n._(msg`Network error: ${detail.message}`);
    case "not_found":
      return i18n._(msg`Not found: ${detail.message}`);
    case "invalid":
      return i18n._(msg`Cannot do that: ${detail.message}`);
    case "missing_market_data":
      return i18n._(msg`No market data for ${detail.key} on ${detail.date} or earlier`);
    case "math":
      return i18n._(msg`The calculation did not converge: ${detail.message}`);
    case "auth":
      return i18n._(msg`The provider did not accept your API key — check it in Settings.`);
    case "rate_limit":
      return detail.retry_after === null
        ? i18n._(msg`The provider is rate limiting you. Try again shortly.`)
        : i18n._(msg`The provider is rate limiting you. Try again in ${detail.retry_after} s.`);
    case "provider":
      return i18n._(msg`The provider returned an error: ${detail.message}`);
    case "refused":
      return i18n._(msg`The model declined to answer this request.`);
    case "truncated":
      return i18n._(msg`The answer was cut off at the model's output limit.`);
    case "locked":
      return i18n._(msg`This profile is locked. Enter its password to continue.`);
    case "password_required":
      return i18n._(msg`Set a password for this profile first: keys are kept only behind one.`);
    case "wrong_password":
      return i18n._(msg`That password is not right.`);
    case "busy":
      return i18n._(
        msg`A quote refresh or an AI answer is still using the data. Try again when it finishes.`,
      );
    case "internal":
      return i18n._(msg`Internal error: ${detail.message}`);
  }
}

export function useErrorText(error: unknown): string {
  const plain = error instanceof Error ? error.message : String(error);
  // Hooks are unconditional: the fallback is chosen after the call, never around it.
  const detail = error instanceof ApiError ? error.detail : INTERNAL_BLANK;
  const localized = useUiErrorText(detail);
  return error instanceof ApiError ? localized : plain;
}

/** Stands in while `useUiErrorText` runs for a failure that is not an `ApiError`. */
const INTERNAL_BLANK: UiError = { code: "internal", message: "" };

/** Query failure as a banner; standalone for the `Page` banner slot. */
export function QueryError({ error }: { error: unknown }) {
  return <Banner tone="bad">{useErrorText(error)}</Banner>;
}

export interface AsyncProps<T> {
  query: UseQueryResult<T>;
  /** Shown instead of `children` when the answer arrived but holds nothing. */
  empty?: ReactNode;
  /** Defaults to "an array with no items"; override for wrapped payloads. */
  isEmpty?: (data: T) => boolean;
  /** Replaces the default wait line where a screen needs its own placeholder. */
  pending?: ReactNode;
  children: (data: T) => ReactNode;
}

/**
 * The three states of a query in one place: failure, waiting, answer.
 * A query disabled by a missing precondition stays pending forever — guard
 * that before rendering `Async`, not inside it.
 */
export function Async<T>({ query, empty, isEmpty, pending, children }: AsyncProps<T>) {
  if (query.isError) return <QueryError error={query.error} />;
  if (query.data === undefined) return <>{pending ?? <Pending />}</>;

  const blank = isEmpty ? isEmpty(query.data) : Array.isArray(query.data) && query.data.length === 0;
  if (blank && empty !== undefined) return <>{empty}</>;

  return <>{children(query.data)}</>;
}
