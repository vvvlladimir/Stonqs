import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { slotFor, SLOT_COUNT } from "../../lib/plot";
import { bucketLabel, slotOf } from "../../lib/taxonomy";
import type {
  AllocationBucket,
  AllocationTarget,
  SubjectKind,
  TaxonomyData,
  TaxonomyNode,
  TaxonomyPreview,
} from "../../lib/types";

export function views(i18n: I18n) {
  return [
    { value: "map", label: i18n._(msg`Map`) },
    { value: "tree", label: i18n._(msg`Tree`) },
    { value: "rings", label: i18n._(msg`Rings`) },
    { value: "bar", label: i18n._(msg`Bar`) },
    { value: "target", label: i18n._(msg`Target`) },
  ];
}

/** Core key for the bucket that keeps allocation totals at 100%. */
export const UNCLASSIFIED = "UNCLASSIFIED";

/** Palette slots shared by the allocation charts. */
export const SLOTS = Array.from({ length: SLOT_COUNT }, (_, i) => slotFor(i));

export interface LevelRow {
  kind: "node" | "position";
  subjectKind?: SubjectKind;
  key: string;
  label: string;
  sub?: string;
  value: string;
  weight: string;
  slot: number;
}

/** Convert portfolio-relative weights to the current parent's scale. */
export function levelShare(weight: string, here: AllocationBucket | null): string {
  if (!here) return weight;
  const whole = Number(here.weight);
  if (!(whole > 0)) return "0";
  return String(Number(weight) / whole);
}

export function nodeSlot(taxonomy: TaxonomyData | null, bucket: AllocationBucket, i: number): number {
  const node = taxonomy?.nodes.find((n) => n.id === bucket.key);
  if (node && taxonomy) return slotOf(taxonomy, node);
  return bucket.key === UNCLASSIFIED ? SLOT_COUNT : slotFor(i);
}

export function nodeSlotById(taxonomy: TaxonomyData, id: string): number | undefined {
  const node = taxonomy.nodes.find((n) => n.id === id);
  return node ? slotOf(taxonomy, node) : undefined;
}

export function descend(buckets: AllocationBucket[], path: string[]): AllocationBucket[] {
  let level = buckets;
  for (const key of path) {
    const next = level.find((b) => b.key === key);
    if (!next) return level;
    level = next.children;
  }
  return level;
}

export function bucketAt(buckets: AllocationBucket[], path: string[]): AllocationBucket | null {
  let level = buckets;
  let current: AllocationBucket | null = null;
  for (const key of path) {
    const next = level.find((b) => b.key === key);
    if (!next) break;
    current = next;
    level = next.children;
  }
  return current;
}

export function labelOf(i18n: I18n, buckets: AllocationBucket[], path: string[]): string {
  const bucket = bucketAt(buckets, path);
  return bucket ? bucketLabel(i18n, bucket) : "";
}

/** Whether enabled securities have an unallocated share. */
export function hasGaps(taxonomy: TaxonomyData, securityIds: string[]): boolean {
  // Excluded subjects are intentionally outside the allocation denominator.
  return securityIds
    .filter((id) => !taxonomy.excluded.includes(id))
    .some((id) => assignedShare(taxonomy, id) < 0.9999);
}

export function assignedShare(taxonomy: TaxonomyData, subjectId: string): number {
  return taxonomy.classifications
    .filter((c) => c.subject_id === subjectId)
    .reduce((sum, c) => sum + Number(c.weight), 0);
}

export function nodeName(taxonomy: TaxonomyData, node: TaxonomyNode): string {
  const parent = taxonomy.nodes.find((n) => n.id === node.parent_id);
  return parent ? `${parent.name} · ${node.name}` : node.name;
}

export function ancestorFactor(
  taxonomy: TaxonomyData,
  target: AllocationTarget | null,
  parent: TaxonomyNode | null,
): number {
  let factor = 1;
  let current: TaxonomyNode | undefined = parent ?? undefined;
  const seen = new Set<string>();
  while (current && !seen.has(current.id)) {
    seen.add(current.id);
    const weight = targetOf(target, current.id);
    if (weight) factor *= Number(weight);
    current = taxonomy.nodes.find((n) => n.id === current!.parent_id);
  }
  return factor;
}

export function targetOf(target: AllocationTarget | null, nodeId: string): string | null {
  return target?.weights.find((w) => w.node_id === nodeId)?.weight ?? null;
}

export function depthOf(preview: TaxonomyPreview): number {
  return preview.nodes.reduce((max, node) => Math.max(max, node.path.length), 0);
}

export function countBy(assignments: TaxonomyPreview["assignments"], how: string): number {
  return assignments.filter((a) => (a.matched_by ?? "").startsWith(how)).length;
}
