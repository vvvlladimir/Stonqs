/** The transaction and price import wizard, and instrument lookup for it. */

import type {
  ImportMapping,
  ImportTemplate,
  ImportOptions,
  ImportPreviewData,
  ImportResult,
  ParseConfig,
  PriceImport,
  RowOverride,
  SecurityDraft,
  Unlock,
} from "../types";
import { call } from "./core";

export const importApi = {
  /** `unlock` answers a `file_protected` failure: the password, for the reader that asked. */
  importLoadPath: (path: string, unlock?: Unlock) =>
    call<ImportPreviewData>("import_load_path", { path, unlock: unlock ?? null }),
  importPreview: (config: ParseConfig, mapping: ImportMapping | null, overrides: RowOverride[]) =>
    call<ImportPreviewData>("import_preview", { config, mapping, overrides }),
  importCommit: (
    config: ParseConfig,
    mapping: ImportMapping | null,
    overrides: RowOverride[],
    options: ImportOptions,
  ) => call<ImportResult>("import_commit", { config, mapping, overrides, options }),
  importClear: () => call<void>("import_clear"),

  /** Resolves one imported security through the network-backed directory. */
  importResolveSymbol: (value: string, isin: string | null, name: string | null, currency: string | null) =>
    call<SecurityDraft | null>("import_resolve_symbol", { value, isin, name, currency }),

  importTemplates: () => call<ImportTemplate[]>("import_templates_list"),
  importTemplateSave: (name: string, config: ParseConfig, mapping: ImportMapping) =>
    call<ImportTemplate[]>("import_template_save", { name, config, mapping }),
  importTemplateDelete: (id: string) => call<ImportTemplate[]>("import_template_delete", { id }),
  importPresetsRestore: () => call<ImportTemplate[]>("import_presets_restore"),

  importPricesLoadPath: (path: string) => call<PriceImport>("import_prices_load_path", { path }),
  importPricesCommit: (config: ParseConfig, mapping: null) =>
    call<number>("import_prices_commit", { config, mapping }),
};
