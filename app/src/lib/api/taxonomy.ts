/** Classification trees, their files, targets and rebalancing. */

import type {
  AllocationTarget,
  RebalancePlan,
  Taxonomy,
  TaxonomyData,
  TaxonomyPreview,
  AttributePreview,
  AttributeImportResult,
  DateString,
} from "../types";
import { call, type Source } from "./core";

export const taxonomyApi = {
  taxonomiesList: () => call<TaxonomyData[]>("taxonomies_list"),
  /** The kind is not asked for: absent keeps what a tree has and makes a new one custom. */
  taxonomySave: (input: { id: string | null; name: string }) => call<unknown>("taxonomy_save", { input }),
  taxonomyDelete: (id: string) => call<void>("taxonomy_delete", { id }),
  taxonomyImportPreviewPath: (path: string, name?: string | null) =>
    call<TaxonomyPreview>("taxonomy_import_preview_path", { path, config: null, name: name ?? null }),
  /** The same preview over bytes, which is what a classification set from a plugin arrives as. */
  taxonomyImportPreview: (content: number[], name?: string | null) =>
    call<TaxonomyPreview>("taxonomy_import_preview", { content, config: null, name: name ?? null }),
  taxonomyImportCommit: (
    content: number[],
    name: string | null,
    into: string | null,
    with_targets: boolean,
  ) =>
    call<Taxonomy>("taxonomy_import_commit", {
      content,
      config: null,
      name,
      into,
      withTargets: with_targets,
    }),
  taxonomyImportCommitPath: (path: string, name: string | null, into: string | null, with_targets: boolean) =>
    call<Taxonomy>("taxonomy_import_commit_path", {
      path,
      config: null,
      name,
      into,
      withTargets: with_targets,
    }),
  /** Plans a tree from one attribute: a node per distinct value. */
  taxonomyGroupPreview: (attribute_id: string, into: string | null) =>
    call<TaxonomyPreview>("taxonomy_group_preview", { attributeId: attribute_id, into }),
  taxonomyGroupCommit: (attribute_id: string, into: string | null, name: string | null) =>
    call<Taxonomy>("taxonomy_group_commit", { attributeId: attribute_id, into, name }),
  taxonomyExportSave: (taxonomy_id: string, path: string) =>
    call<void>("taxonomy_export_save", { taxonomyId: taxonomy_id, path }),
  /** What an attribute CSV would fill in, before it fills anything in. */
  attributesImportPreviewPath: (path: string) =>
    call<AttributePreview>("attributes_import_preview_path", { path, config: null }),
  attributesImportCommitPath: (path: string) =>
    call<AttributeImportResult>("attributes_import_commit_path", { path, config: null }),
  /** Writes every instrument's attributes as the CSV this same import reads back. */
  attributesExportSave: (path: string) => call<void>("attributes_export_save", { path }),
  taxonomyNodeSave: (input: {
    color?: number | null;
    id: string | null;
    taxonomy_id: string;
    parent_id: string | null;
    name: string;
    rank: number;
  }) => call<unknown>("taxonomy_node_save", { input }),
  taxonomyNodeDelete: (id: string) => call<void>("taxonomy_node_delete", { id }),
  classificationSave: (input: { subject_id: string; node_id: string; weight: string }) =>
    call<void>("classification_save", { input }),
  classificationDelete: (subject_id: string, node_id: string) =>
    call<void>("classification_delete", { subjectId: subject_id, nodeId: node_id }),
  /** Excludes or restores a subject without changing its assignments. */
  taxonomyExclude: (taxonomy_id: string, subject_id: string, excluded: boolean) =>
    call<void>("taxonomy_exclude", { taxonomyId: taxonomy_id, subjectId: subject_id, excluded }),

  targetsList: () => call<AllocationTarget[]>("targets_list"),
  targetSave: (input: {
    id: string | null;
    taxonomy_id: string;
    name: string;
    weights: Array<[string, string]>;
  }) => call<AllocationTarget>("target_save", { input }),
  targetDelete: (id: string) => call<void>("target_delete", { id }),
  /** Cash and sell options change the calculation, not the displayed layout. */
  rebalancePlan: (
    target_id: string,
    date: DateString,
    cash_to_invest?: string | null,
    allow_sell?: boolean,
    source?: Source,
  ) =>
    call<RebalancePlan>("rebalance_plan", {
      targetId: target_id,
      date,
      cashToInvest: cash_to_invest ?? null,
      allowSell: allow_sell ?? true,
      source: source ?? null,
    }),
};
