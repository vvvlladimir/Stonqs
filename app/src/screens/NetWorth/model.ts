import type { Asset, AssetInput, AssetKind, DateString } from "../../lib/types";
import { isOwedKind } from "../../lib/kinds";

/** A blank asset, given a kind and a currency by the button that opens the form. */
export function blankAsset(kind: AssetKind, currency: string, today: DateString): AssetInput {
  return {
    id: null,
    name: "",
    kind,
    currency,
    secured_by: null,
    note: null,
    closed_on: null,
    rate: null,
    monthly_payment: null,
    ends_on: null,
    amount: "",
    valued_on: today,
  };
}

/** An existing asset as the form edits it. No value fields: editing a name is not a valuation. */
export function assetToInput(asset: Asset): AssetInput {
  return {
    id: asset.id,
    name: asset.name,
    kind: asset.kind,
    currency: asset.currency,
    secured_by: asset.secured_by,
    note: asset.note,
    closed_on: asset.closed_on,
    // Stored as a fraction, typed as a percent — the same trick the goal form plays.
    rate: asset.schedule ? String(Number(asset.schedule.rate) * 100) : null,
    monthly_payment: asset.schedule?.monthly_payment ?? null,
    ends_on: asset.schedule?.ends_on ?? null,
    amount: null,
    valued_on: null,
  };
}

/** The window the line is drawn over: whole years back from the reading date. */
export function windowFrom(date: DateString, years: number): DateString {
  const year = Number(date.slice(0, 4)) - years;
  return `${year}${date.slice(4)}`;
}

export function isOwed(input: AssetInput): boolean {
  return isOwedKind(input.kind);
}
