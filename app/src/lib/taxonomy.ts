import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { slotFor } from "./plot";
import type { TaxonomyData, TaxonomyNode } from "./types";

/** Buckets the core generates instead of reading a name from the tree. */
export const UNCLASSIFIED_KEY = "UNCLASSIFIED";
export const CASH_KEY = "CASH";

/**
 * A bucket's display name. The core labels its two generated buckets with their key,
 * because naming them would mean choosing a language inside the calculation.
 */
export function bucketLabel(i18n: I18n, bucket: { key: string; label: string }): string {
  if (bucket.key === UNCLASSIFIED_KEY) return i18n._(msg`not classified`);
  if (bucket.key === CASH_KEY) return i18n._(msg`cash`);
  return bucket.label;
}

/**
 * Category color: explicit slot, else root order (so an uncolored tree still
 * shows distinct neighbors). Lives in lib, not the allocation screen — the
 * color is a property of the tree, shared by every chart that renders it.
 */
export function slotOf(taxonomy: TaxonomyData, node: TaxonomyNode): number {
  let current: TaxonomyNode | undefined = node;
  if (current.color) return current.color;
  while (current && current.parent_id) {
    current = taxonomy.nodes.find((n) => n.id === current!.parent_id);
  }
  const roots = taxonomy.nodes.filter((n) => n.parent_id === null);
  const index = roots.findIndex((r) => r.id === current?.id);
  return slotFor(index < 0 ? 0 : index);
}

/** Slot by node id; the node may no longer exist, hence the optional return. */
export function slotOfNode(taxonomy: TaxonomyData | undefined, nodeId: string): number | undefined {
  if (!taxonomy) return undefined;
  const node = taxonomy.nodes.find((n) => n.id === nodeId);
  return node ? slotOf(taxonomy, node) : undefined;
}

/** Converts a fractional weight to an input percentage without multiplication. */
export function weightToPercent(weight: string | undefined): string {
  if (!weight) return "";
  const match = /^([+-]?)(\d*)(?:\.(\d*))?$/.exec(weight.trim());
  if (!match) return "";
  const digits = (match[2] || "0") + (match[3] ?? "");
  const point = (match[2] || "0").length + 2;
  const padded = digits.padEnd(point, "0");
  const int = padded.slice(0, point).replace(/^0+(?=\d)/, "");
  const frac = padded.slice(point).replace(/0+$/, "");
  return frac ? `${match[1]}${int}.${frac}` : `${match[1]}${int}`;
}

/** Converts an input percentage back to a fractional weight. */
export function percentToWeight(percent: string | undefined): string {
  const match = /^(\d*)(?:[.,](\d*))?$/.exec((percent ?? "").trim());
  if (!match) return "0";
  const int = match[1] || "0";
  const frac = match[2] ?? "";
  const digits = int + frac;
  const point = int.length - 2;
  if (point <= 0) return `0.${"0".repeat(-point)}${digits}`;
  return `${digits.slice(0, point)}.${digits.slice(point)}`;
}

/** Full path of a node, for a picker that must tell two same-named leaves apart. */
export function branchName(taxonomy: TaxonomyData, node: TaxonomyNode): string {
  const path = [node.name];
  let current: TaxonomyNode | undefined = node;
  while (current?.parent_id) {
    current = taxonomy.nodes.find((n) => n.id === current!.parent_id);
    if (current) path.unshift(current.name);
  }
  return path.join(" › ");
}
