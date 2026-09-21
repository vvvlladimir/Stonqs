/** Classification trees and the breakdown they produce. */

import type { ImportProblem } from "./imports";
import type { MoneyString } from "./primitives";
export interface AllocationBucket {
  key: string;
  label: string;
  value_base: MoneyString;
  weight: MoneyString;
  children: AllocationBucket[];
}

export interface Allocation {
  total_base: MoneyString;
  buckets: AllocationBucket[];
}

export type TaxonomyKind = "ASSET_CLASS" | "REGION" | "SECTOR" | "CUSTOM";

export interface TaxonomyNode {
  id: string;
  taxonomy_id: string;
  parent_id: string | null;
  name: string;
  rank: number;
  /** Palette slot 1–8; null falls back to node order. */
  color: number | null;
}

/** Security or account cash represented in an allocation tree. */
export type SubjectKind = "SECURITY" | "CASH";

/** Subject allocation inside one taxonomy node. */
export interface NodeMember {
  /** Security ID or `cash:<account>:<currency>`. */
  subject_id: string;
  kind: SubjectKind;
  symbol: string;
  name: string;
  value_base: MoneyString;
  /** Weight within the level; excluded subjects have `"0"`. */
  weight: MoneyString;
  subject_value_base: MoneyString;
  /** Assigned fraction of the subject. */
  assigned_share: MoneyString;
  /** Excluded from this taxonomy's calculations. */
  excluded: boolean;
}

/** Assignment of one subject to one taxonomy node. */
export interface Assignment {
  subject_id: string;
  node_id: string;
  weight: MoneyString;
}

/** Taxonomy metadata returned by save commands. */
export interface Taxonomy {
  id: string;
  name: string;
  kind: TaxonomyKind;
}

export interface TaxonomyData {
  id: string;
  name: string;
  kind: TaxonomyKind;
  nodes: TaxonomyNode[];
  classifications: Assignment[];
  /** Subjects excluded from this taxonomy. */
  excluded: string[];
}

/** Preview configuration and result for a taxonomy CSV import. */
export interface TaxonomyCsvConfig {
  levels: string[];
  weight: string | null;
  target: string | null;
  symbol: string | null;
  isin: string | null;
  /** Treat the first level as the taxonomy name. */
  root_is_name: boolean;
}

export interface TaxonomyPreviewNode {
  path: string[];
  /** Target weight as a portfolio fraction. */
  target: MoneyString | null;
}

export interface TaxonomyPreviewAssignment {
  row: number;
  path: string[];
  label: string;
  symbol: string;
  isin: string;
  weight: MoneyString;
  /** Matched security ID, or null when absent from the database. */
  security_id: string | null;
  /** Match source, such as ISIN, ticker, or name. */
  matched_by: string | null;
}

export interface TaxonomyPreview {
  name: string;
  kind: TaxonomyKind;
  config: TaxonomyCsvConfig;
  nodes: TaxonomyPreviewNode[];
  assignments: TaxonomyPreviewAssignment[];
  problems: ImportProblem[];
}
