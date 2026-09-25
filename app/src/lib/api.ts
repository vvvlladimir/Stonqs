/** The only frontend module that imports Tauri APIs. */

import { getVersion } from "@tauri-apps/api/app";
import { Channel, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu } from "@tauri-apps/api/menu";
import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import type {
  AlertInput,
  AlertRow,
  CrossingRow,
  DevAlertStep,
  SecurityAlert,
  SecurityEvent,
  SecurityEventInput,
  SecurityEventRow,
  AttributeDefInput,
  SecurityAttributeDef,
  PaymentPeriod,
  PaymentsData,
  ExpectedDividendsData,
  CorporateAction,
  CorporateActionInput,
  CorporateActionRow,
  InvestmentPlan,
  PlanDue,
  PlanInput,
  PlanProjection,
  FireProjection,
  PlansData,
  Account,
  Allocation,
  NodeMember,
  AllocationTarget,
  BenchmarkComparison,
  PerformanceData,
  CalculationSheet,
  SheetPeriod,
  PeriodRange,
  PeriodSettings,
  UserPeriod,
  PositionReturn,
  PositionReturnRow,
  PositionsData,
  PositionCostData,
  ImportMapping,
  ImportTemplate,
  ImportOptions,
  ImportPreviewData,
  ImportResult,
  ParseConfig,
  PriceImport,
  Quote,
  RebalancePlan,
  AppSettings,
  Profile,
  Plugin,
  PluginList,
  ProfileList,
  DataCoverage,
  Progress,
  RefreshMode,
  RefreshStatus,
  MarketSourceRow,
  CustomSource,
  CustomTestRow,
  ReportsData,
  TradesData,
  IncomeData,
  TaxonomyIncomeData,
  RowOverride,
  RiskReport,
  GrowthSeries,
  InflationStatus,
  SecurityDraft,
  SecurityMatch,
  Taxonomy,
  TaxonomyData,
  TransactionKind,
  TaxonomyPreview,
  Goal,
  GoalInput,
  GoalRow,
  LimitInput,
  LimitUsage,
  AttributePreview,
  AttributeImportResult,
  Transaction,
  TransactionFilter,
  TransactionInput,
  TransactionsData,
  TransferSuggestion,
  AccountInput,
  AccountRow,
  AccountsTotal,
  AccountGroup,
  AccountGroupInput,
  AccountGroupRow,
  DataScope,
  ScopeState,
  AiChat,
  AiEvent,
  AiProvider,
  AiToolMode,
  AiEffort,
  AiUsageTotal,
  ToolDecision,
  AppStatus,
  ChatMessage,
  DataChanged,
  DataChangeKind,
  DashboardData,
  DateString,
  Portfolio,
  PortfolioInput,
  RealPerformance,
  Security,
  SecurityInput,
  Listing,
  ListingChoice,
  SecurityRow,
  SetupInput,
  UiError,
  Watchlist,
  WatchlistInput,
  WatchRow,
} from "./types";
import type { Outcome } from "./ipcRecord";

// Folded to `undefined` unless recording, so a normal build does not even carry the chunk.
const record = import.meta.env.VITE_RECORD_IPC
  ? async (command: string, args: Record<string, unknown> | undefined, outcome: Outcome) => {
      const { recordIpc } = await import("./ipcRecord");
      recordIpc(command, args, outcome);
    }
  : undefined;

/** Normalizes serialized host errors while preserving their structured detail. */
export class ApiError extends Error {
  constructor(readonly detail: UiError) {
    super(detail.message);
    this.name = "ApiError";
  }
}

function isUiError(value: unknown): value is UiError {
  return typeof value === "object" && value !== null && "code" in value && "message" in value;
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const result = await invoke<T>(command, args);
    void record?.(command, args, { ok: result });
    return result;
  } catch (raw) {
    void record?.(command, args, { err: raw });
    if (isUiError(raw)) throw new ApiError(raw);
    throw new ApiError({ code: "internal", message: String(raw) });
  }
}

/**
 * The data source a reporting call is answered in. Absent — the usual case — means the one the
 * picker holds; a dashboard widget names its own so a board can carry a tile per account.
 */
export type Source = DataScope | null | undefined;

export const api = {
  appStatus: () => call<AppStatus>("app_status"),
  dashboardSummary: (date: DateString, source?: Source) =>
    call<DashboardData>("dashboard_summary", { date, source: source ?? null }),

  portfolioGet: () => call<Portfolio>("portfolio_get"),
  portfolioSave: (input: PortfolioInput) => call<Portfolio>("portfolio_save", { input }),
  setupPortfolio: (input: SetupInput) => call<Portfolio>("setup_portfolio", { input }),

  accountsList: () => call<AccountRow[]>("accounts_list"),
  /** Core-computed total for all accounts. */
  accountsTotal: () => call<AccountsTotal>("accounts_total"),
  accountSave: (input: AccountInput) => call<Account>("account_save", { input }),
  accountDelete: (id: string, force: boolean) => call<void>("account_delete", { id, force }),

  accountGroupsList: () => call<AccountGroupRow[]>("account_groups_list"),
  accountGroupSave: (input: AccountGroupInput) => call<AccountGroup>("account_group_save", { input }),
  accountGroupDelete: (id: string) => call<void>("account_group_delete", { id }),

  /** Current data scope and available scope options. */
  scopeGet: () => call<ScopeState>("scope_get"),
  scopeSet: (scope: DataScope) => call<DataScope>("scope_set", { scope }),

  securitiesList: () => call<SecurityRow[]>("securities_list"),
  securitySave: (input: SecurityInput) => call<Security>("security_save", { input }),
  securityDelete: (id: string) => call<void>("security_delete", { id }),
  /** The attributes the user defined; their values travel on the security row. */
  attributeDefsList: () => call<SecurityAttributeDef[]>("attribute_defs_list"),
  attributeDefSave: (input: AttributeDefInput) => call<SecurityAttributeDef>("attribute_def_save", { input }),
  attributeDefDelete: (id: string) => call<void>("attribute_def_delete", { id }),
  /** Splits of one instrument, or of every instrument when the id is omitted. */
  corporateActionsList: (security_id?: string) =>
    call<CorporateActionRow[]>("corporate_actions_list", { securityId: security_id ?? null }),
  corporateActionSave: (input: CorporateActionInput) =>
    call<CorporateAction>("corporate_action_save", { input }),
  corporateActionDelete: (id: string) => call<void>("corporate_action_delete", { id }),
  /** Limit prices and date rules with what each says today; every instrument without an id. */
  alertsList: (security_id?: string) => call<AlertRow[]>("alerts_list", { securityId: security_id ?? null }),
  alertSave: (input: AlertInput) => call<SecurityAlert>("alert_save", { input }),
  alertDelete: (id: string) => call<void>("alert_delete", { id }),
  /** The crossing log across every rule, newest first. */
  alertCrossingsList: (limit?: number) =>
    call<CrossingRow[]>("alert_crossings_list", { limit: limit ?? null }),
  /** How many crossings were not looked at; the navigation shows a dot while any are left. */
  alertsUnseen: () => call<number>("alerts_unseen"),
  alertsMarkSeen: () => call<number>("alerts_mark_seen"),
  /** Checks every rule, then hands over the crossings not announced yet, marked as announced. */
  alertsTakeNotifications: () => call<CrossingRow[]>("alerts_take_notifications"),
  /** Notes, dividends and splits, newest first; every instrument without an id. */
  securityEventsList: (security_id?: string) =>
    call<SecurityEventRow[]>("security_events_list", { securityId: security_id ?? null }),
  securityEventSave: (input: SecurityEventInput) => call<SecurityEvent>("security_event_save", { input }),
  securityEventDelete: (id: string) => call<void>("security_event_delete", { id }),
  /** Named lists of instruments; not scoped, like the alerts on them. */
  watchlistsList: () => call<Watchlist[]>("watchlists_list"),
  watchlistSave: (input: WatchlistInput) => call<Watchlist>("watchlist_save", { input }),
  watchlistDelete: (id: string) => call<void>("watchlist_delete", { id }),
  /** Each instrument's own price facts over the period, in the list's order. */
  watchlistRows: (id: string, from: DateString, to: DateString) =>
    call<WatchRow[]>("watchlist_rows", { id, from, to }),
  /** Regular contributions. A plan proposes transactions; nothing is written until committed. */
  plansList: () => call<PlansData>("plans_list"),
  planSave: (input: PlanInput) => call<InvestmentPlan>("plan_save", { input }),
  planDelete: (id: string) => call<void>("plan_delete", { id }),
  /** Occurrences up to today with nothing committed against them, drafts included. */
  planDue: (plan_id: string, as_of?: DateString) =>
    call<PlanDue[]>("plan_due", { planId: plan_id, asOf: as_of ?? null }),
  /** Writes the confirmed rows and records the occurrence; returns how many were written. */
  planCommit: (plan_id: string, date: DateString, drafts: TransactionInput[]) =>
    call<number>("plan_commit", { planId: plan_id, date, drafts }),
  planProjection: (months: number) => call<PlanProjection>("plan_projection", { months }),
  /** The FIRE reading: every figure but today's value is an assumption the user typed. */
  fireProjection: (assumptions: {
    annual_spending: string;
    withdrawal_rate: string;
    expected_return: string;
    contribution: string | null;
  }) =>
    call<FireProjection>("fire_projection", {
      annualSpending: assumptions.annual_spending,
      withdrawalRate: assumptions.withdrawal_rate,
      expectedReturn: assumptions.expected_return,
      contribution: assumptions.contribution,
    }),
  /** Identifies an existing security and updates its market metadata. */
  securityIdentify: (id: string) => call<Security>("security_identify", { id }),
  /** Cached quotes for a period; this command does not use the network. */
  quotesRange: (security_id: string, from: DateString, to: DateString) =>
    call<Quote[]>("quotes_range", { securityId: security_id, from, to }),
  /** Security venues; `refresh` forces a new directory lookup. */
  securityListings: (id: string, refresh: boolean) => call<Listing[]>("security_listings", { id, refresh }),
  /** Changes the listing and clears quotes for the previous symbol. */
  securitySetListing: (choice: ListingChoice) => call<Security>("security_set_listing", { choice }),

  /** The same positions' purchase value under both cost-basis methods. */
  positionsCostBasis: (date: DateString, source?: Source) =>
    call<PositionCostData>("positions_cost_basis", { date, source: source ?? null }),
  positionsAt: (date: DateString, source?: Source) =>
    call<PositionsData>("positions_at", { date, source: source ?? null }),
  positionReturn: (security_id: string, from: DateString, to: DateString, source?: Source) =>
    call<PositionReturn>("position_return", { securityId: security_id, from, to, source: source ?? null }),
  /** Returns and contributions for all positions in one history pass. */
  positionReturns: (from: DateString, to: DateString, source?: Source) =>
    call<PositionReturnRow[]>("position_returns", { from, to, source: source ?? null }),

  transactionsList: (filter: TransactionFilter) => call<TransactionsData>("transactions_list", { filter }),
  transactionsExportSave: (filter: TransactionFilter, path: string) =>
    call<void>("transactions_export_save", { filter, path }),
  transactionSave: (input: TransactionInput) => call<Transaction>("transaction_save", { input }),
  transactionDelete: (id: string) => call<void>("transaction_delete", { id }),
  /** Moves between two of the user's own accounts that arrived as two unrelated rows. */
  transferSuggestions: () => call<TransferSuggestion[]>("transfer_suggestions"),
  transferLink: (ids: [string, string]) => call<void>("transfer_link", { ids }),

  periodRanges: (as_of: DateString) => call<PeriodRange[]>("period_ranges", { asOf: as_of }),
  periodsGet: () => call<PeriodSettings>("periods_get"),
  periodSave: (period: UserPeriod) => call<PeriodSettings>("period_save", { period }),
  /** Deletes the user's own period; a shipped preset is only hidden. */
  periodDelete: (id: string) => call<PeriodSettings>("period_delete", { id }),
  periodsRestore: () => call<PeriodSettings>("periods_restore"),
  performanceSummary: (from: DateString, to: DateString, source?: Source) =>
    call<PerformanceData>("performance_summary", { from, to, source: source ?? null }),
  /** The calculation sheet: one row per calendar chunk of the same period. */
  performanceBreakdown: (from: DateString, to: DateString, period: SheetPeriod, source?: Source) =>
    call<CalculationSheet>("performance_breakdown", { from, to, period, source: source ?? null }),
  /** Writes the same sheet as a CSV where the user points. */
  performanceSheetSave: (from: DateString, to: DateString, period: SheetPeriod, path: string) =>
    call<void>("performance_sheet_save", { from, to, period, path }),
  /** The payments grid: every dated line of the window on one axis. */
  paymentsGrid: (from: DateString, to: DateString, period: PaymentPeriod, source?: Source) =>
    call<PaymentsData>("payments_grid", { from, to, period, source: source ?? null }),
  /** Dividends the open positions should pay over the next `months` months, at today's rates. */
  dividendsExpected: (months: number, source?: Source) =>
    call<ExpectedDividendsData>("dividends_expected", { months, source: source ?? null }),
  /** Open and closed trades of a window, with the turnover the trading produced. */
  tradesSummary: (from: DateString, to: DateString, source?: Source) =>
    call<TradesData>("trades_summary", { from, to, source: source ?? null }),
  /** Full risk report; rolling-volatility window is in trading days. */
  riskReport: (from: DateString, to: DateString, risk_free_rate: number, window_days = 63, source?: Source) =>
    call<RiskReport>("risk_report", {
      from,
      to,
      riskFreeRate: risk_free_rate,
      windowDays: window_days,
      source: source ?? null,
    }),
  benchmarkCompare: (security_id: string, from: DateString, to: DateString, source?: Source) =>
    call<BenchmarkComparison>("benchmark_compare", {
      securityId: security_id,
      from,
      to,
      source: source ?? null,
    }),
  /** Benchmark growth aligned to the portfolio dates. */
  benchmarkSeries: (security_id: string, from: DateString, to: DateString, source?: Source) =>
    call<GrowthSeries>("benchmark_series", { securityId: security_id, from, to, source: source ?? null }),

  /** Region, source and how far the stored index reaches. */
  inflationStatus: () => call<InflationStatus>("inflation_status"),

  /** Points the portfolio at a price-index region, or switches real returns off with `null`. */
  inflationRegionSet: (region: string | null) => call<void>("inflation_region_set", { region }),

  /** The period's returns with inflation taken out; `null` when no region is set. */
  realPerformance: (from: DateString, to: DateString, source?: Source) =>
    call<RealPerformance | null>("real_performance", { from, to, source: source ?? null }),

  /** Cost of one unit of money across the period, based at its first day. */
  inflationSeries: (from: DateString, to: DateString, source?: Source) =>
    call<GrowthSeries | null>("inflation_series", { from, to, source: source ?? null }),

  allocation: (cut: string, taxonomy_id: string | null, date: DateString, source?: Source) =>
    call<Allocation>("allocation", { cut, taxonomyId: taxonomy_id, date, source: source ?? null }),

  /** Members and assigned portions for one allocation level. */
  allocationMembers: (taxonomy_id: string, node_id: string | null, date: DateString, source?: Source) =>
    call<NodeMember[]>("allocation_members", {
      taxonomyId: taxonomy_id,
      nodeId: node_id,
      date,
      source: source ?? null,
    }),

  /** Full category tree for the full-depth hierarchy view. */
  allocationTree: (taxonomy_id: string, date: DateString, source?: Source) =>
    call<Allocation>("allocation_tree", { taxonomyId: taxonomy_id, date, source: source ?? null }),

  /** Goals with their progress. Not scoped: a goal carries the accounts it counts. */
  goalsList: (date: DateString) => call<GoalRow[]>("goals_list", { date }),
  goalSave: (input: GoalInput) => call<Goal>("goal_save", { input }),
  goalDelete: (id: string) => call<void>("goal_delete", { id }),
  /** Contribution limits with what the limit year `date` falls in has taken. */
  limitsList: (date: DateString) => call<LimitUsage[]>("limits_list", { date }),
  limitSave: (input: LimitInput) => call<unknown>("limit_save", { input }),
  limitDelete: (id: string) => call<void>("limit_delete", { id }),
  taxonomiesList: () => call<TaxonomyData[]>("taxonomies_list"),
  /** The kind is not asked for: absent keeps what a tree has and makes a new one custom. */
  taxonomySave: (input: { id: string | null; name: string }) => call<unknown>("taxonomy_save", { input }),
  taxonomyDelete: (id: string) => call<void>("taxonomy_delete", { id }),
  taxonomyImportPreviewPath: (path: string, name?: string | null) =>
    call<TaxonomyPreview>("taxonomy_import_preview_path", { path, config: null, name: name ?? null }),
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

  reportsSummary: (from: DateString, to: DateString, source?: Source) =>
    call<ReportsData>("reports_summary", { from, to, source: source ?? null }),
  /** Income events and core-computed breakdowns for a period. */
  /** `kind` narrows the report to one income kind; null keeps every kind. */
  incomeSummary: (from: DateString, to: DateString, kind: TransactionKind | null = null, source?: Source) =>
    call<IncomeData>("income_summary", { from, to, kind, source: source ?? null }),
  /** The same period's income split by one classification tree. */
  incomeTaxonomy: (
    taxonomyId: string,
    from: DateString,
    to: DateString,
    kind: TransactionKind | null = null,
    source?: Source,
  ) =>
    call<TaxonomyIncomeData>("income_taxonomy", {
      taxonomyId,
      from,
      to,
      kind,
      source: source ?? null,
    }),
  /** `section` names one table of the report ("gains.detail", "charges.account", …). */
  reportSave: (section: string, from: DateString, to: DateString, path: string) =>
    call<void>("report_save", { section, from, to, path }),

  importLoadPath: (path: string) => call<ImportPreviewData>("import_load_path", { path }),
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
  securitySearch: (query: string) => call<SecurityMatch[]>("security_search", { query }),
  /** The exact listing's profile, currency included; never another listing of the same name. */
  securityProfile: (source: string, symbol: string) =>
    call<SecurityMatch | null>("security_profile", { source, symbol }),
  securityResolve: (query: string, currency: string | null = null) =>
    call<SecurityMatch | null>("security_resolve", { query, currency }),

  importTemplates: () => call<ImportTemplate[]>("import_templates_list"),
  importTemplateSave: (name: string, config: ParseConfig, mapping: ImportMapping) =>
    call<ImportTemplate[]>("import_template_save", { name, config, mapping }),
  importTemplateDelete: (id: string) => call<ImportTemplate[]>("import_template_delete", { id }),
  importPresetsRestore: () => call<ImportTemplate[]>("import_presets_restore"),

  importPricesLoadPath: (path: string) => call<PriceImport>("import_prices_load_path", { path }),
  importPricesCommit: (config: ParseConfig, mapping: null) =>
    call<number>("import_prices_commit", { config, mapping }),

  dataCoverage: () => call<DataCoverage[]>("data_coverage"),
  /** Returns false when a refresh is already running. */
  marketRefresh: (mode: RefreshMode) => call<boolean>("market_refresh", { mode }),
  /** Requests cancellation between securities. */
  marketRefreshCancel: () => call<void>("market_refresh_cancel"),
  refreshStatus: () => call<RefreshStatus>("refresh_status"),
  /** Registered quote providers for the security screen. */
  quoteProviders: () => call<string[]>("quote_providers"),
  marketSources: () => call<MarketSourceRow[]>("market_sources_list"),
  marketSourceSwitch: (source: string, on: boolean) => call<void>("market_source_switch", { source, on }),
  marketKeySave: (source: string, key: string) => call<void>("market_key_save", { source, key }),
  marketKeyDelete: (source: string) => call<void>("market_key_delete", { source }),
  marketCustom: () => call<CustomSource[]>("market_custom_list"),
  marketCustomSave: (source: CustomSource) => call<void>("market_custom_save", { source }),
  marketCustomDelete: (id: string) => call<void>("market_custom_delete", { id }),
  marketCustomTest: (source: CustomSource, symbol: string, currency: string) =>
    call<CustomTestRow[]>("market_custom_test", { source, symbol, currency }),

  settingsGet: () => call<AppSettings>("settings_get"),

  pluginsList: () => call<PluginList>("plugins_list"),
  /** Installs the folder the user picked; a path, because a plugin is a folder, not a file. */
  pluginInstall: (path: string) => call<Plugin>("plugin_install", { path }),
  pluginRemove: (id: string) => call<void>("plugin_remove", { id }),
  pluginThemeCss: (plugin: string, theme: string) => call<string>("plugin_theme_css", { plugin, theme }),

  profilesList: () => call<ProfileList>("profiles_list"),
  profileCreate: (name: string) => call<Profile>("profile_create", { name }),
  profileRename: (id: string, name: string) => call<Profile>("profile_rename", { id, name }),
  /** Deletes the *open* profile (its password, when it has one) and moves to another. */
  profileDelete: (password: string | null) => call<void>("profile_delete", { password }),
  /** Swaps the host's open profile. The caller reloads the window: everything cached is the old one's. */
  profileOpen: (id: string) => call<void>("profile_open", { id }),
  profileUnlock: (password: string, remember: boolean) =>
    call<void>("profile_unlock", { password, remember }),
  profileLock: () => call<void>("profile_lock"),
  /** The first password when `current` is null, a change otherwise. */
  profileSetPassword: (current: string | null, password: string) =>
    call<void>("profile_set_password", { current, password }),
  profileRemovePassword: (current: string) => call<void>("profile_remove_password", { current }),
  profileRemember: (remember: boolean) => call<void>("profile_remember", { remember }),
  settingsSave: (settings: AppSettings) => call<AppSettings>("settings_save", { settings }),
  /** Saves dashboard layout separately from general settings. */
  uiStateSave: (ui: unknown) => call<void>("ui_state_save", { ui }),

  /** No command reads a saved key back — only save, delete, and a connected/not-connected status. */
  aiKeySave: (provider: string, key: string) => call<void>("ai_key_save", { provider, key }),
  aiKeyDelete: (provider: string) => call<void>("ai_key_delete", { provider }),
  aiKeyStatus: (provider: string) => call<boolean>("ai_key_status", { provider }),

  aiChatsList: () => call<AiChat[]>("ai_chats_list"),
  aiChatCreate: (title: string) => call<AiChat>("ai_chat_create", { title }),
  aiChatRename: (id: string, title: string) => call<void>("ai_chat_rename", { id, title }),
  aiChatSetMode: (id: string, mode: AiToolMode) => call<void>("ai_chat_set_mode", { id, mode }),
  aiChatSetEffort: (id: string, effort: AiEffort) => call<void>("ai_chat_set_effort", { id, effort }),
  aiChatSetModel: (id: string, model: string) => call<void>("ai_chat_set_model", { id, model }),
  /** Moving a chat to another provider takes its model with it: a model id belongs to the
   *  catalogue it came from, so the host lands the chat on the new provider's default. */
  aiChatSetProvider: (id: string, provider: string) => call<void>("ai_chat_set_provider", { id, provider }),
  /** The providers this build can talk to. A list of what exists, not of what is connected. */
  aiProvidersList: () => call<AiProvider[]>("ai_providers_list"),
  /** What one provider says it offers, best matches first; nothing is filtered out. */
  aiModelsList: (provider: string) => call<string[]>("ai_models_list", { provider }),
  aiChatDelete: (id: string) => call<void>("ai_chat_delete", { id }),
  /** One chat as Markdown, written to a path the user picked — the way to keep a conversation
   *  after deleting the history. */
  aiChatExportSave: (id: string, path: string) => call<void>("ai_chat_export_save", { id, path }),
  aiMessagesList: (chatId: string) => call<ChatMessage[]>("ai_messages_list", { chatId }),
  /** The tools already allowed for the rest of this chat — never the tools that exist. */
  aiGrantsList: (chatId: string) => call<string[]>("ai_grants_list", { chatId }),
  /** Answers one consent card by id. The host looks the call up; this sends nothing else. */
  aiToolDecide: (requestId: string, decision: ToolDecision) =>
    call<void>("ai_tool_decide", { requestId, decision }),
  aiCancel: () => call<void>("ai_cancel"),

  /** What every model has been asked for so far, counted by the provider, never priced here. */
  aiUsageTotals: () => call<AiUsageTotal[]>("ai_usage_totals"),
  /**
   * Streams the reply through `onEvent` as it arrives; the returned promise settles once the
   * command has been dispatched, not once the turn is over — a rejection here means the call
   * never started (disabled, no key), not that the reply failed. A mid-turn failure arrives as
   * an `error` event instead.
   */
  /** The dashboard brief: a fixed set of readings, one model call, streamed like a chat turn. */
  aiBrief: (
    from: DateString,
    to: DateString,
    source: Source,
    instructions: string | null,
    language: string,
    /** The tile's own; `null` follows where a new chat begins, and the newest model there. */
    model: { provider: string | null; model: string | null },
    /** The answer's length in output tokens; `null` leaves the provider's own ceiling. */
    maxTokens: number | null,
    onEvent: (event: AiEvent) => void,
  ) => {
    const channel = new Channel<AiEvent>();
    channel.onmessage = onEvent;
    return call<void>("ai_brief", {
      from,
      to,
      source: source ?? null,
      instructions,
      language,
      provider: model.provider,
      model: model.model,
      maxTokens,
      onEvent: channel,
    });
  },

  aiSend: (
    chatId: string,
    text: string,
    screen: string | null,
    asOf: string | null,
    onEvent: (event: AiEvent) => void,
  ) => {
    const channel = new Channel<AiEvent>();
    channel.onmessage = onEvent;
    return call<void>("ai_send", { chatId, text, screen, asOf, onEvent: channel });
  },

  /** Fills an empty portfolio with the sample history; refuses once it holds an account. */
  demoSeed: () => call<void>("demo_seed"),
  /** Debug builds only: moves a real quote across a rule's level; returns crossings logged. */
  devAlertSimulate: (alert_id: string, step: DevAlertStep) =>
    call<number>("dev_alert_simulate", { alertId: alert_id, step }),
};

/** Subscribes to refresh progress shared by the whole application. */
export function onMarketProgress(handler: (progress: Progress) => void): Promise<() => void> {
  return listen<Progress>("market:progress", (event) => handler(event.payload));
}

/** The host says what it touched, so the frontend invalidates that group only. */
export function onDataChanged(handler: (kind: DataChangeKind) => void): Promise<() => void> {
  return listen<DataChanged>("data:changed", (event) => handler(event.payload.scope));
}

/** Shows an OS notification, asking for permission the first time. False when it is refused. */
export async function notify(title: string, body: string): Promise<boolean> {
  let granted = await isPermissionGranted();
  if (!granted) granted = (await requestPermission()) === "granted";
  if (granted) sendNotification({ title, body });
  return granted;
}

/**
 * What an available update says about itself. The plugin's own handle stays in this module: a
 * screen decides whether to install, it never holds an installer.
 */
export interface AvailableUpdate {
  version: string;
  /** The release body, as markdown. */
  notes: string;
  /** Publication date exactly as the release names it; absent for a release without one. */
  date: string | null;
}

/** The running build's own version, as the bundle declares it. */
export function appVersion(): Promise<string> {
  return getVersion();
}

let pending: Update | null = null;

/**
 * Asks the release feed whether something newer is signed and published. `null` is the normal
 * answer. Throws when the feed cannot be reached — being offline is not "up to date".
 */
export async function checkForUpdate(): Promise<AvailableUpdate | null> {
  const update = await check();
  pending = update;
  if (!update) return null;
  return { version: update.version, notes: update.body ?? "", date: update.date ?? null };
}

/**
 * Downloads and installs what the last check found, reporting how much has arrived: a fraction
 * while the size is known, `null` while it is not, because a server may send no content length.
 * False when there is nothing pending — the check was superseded, so it is asked again.
 */
export async function installUpdate(onProgress: (done: number | null) => void): Promise<boolean> {
  const update = pending;
  if (!update) return false;
  let total = 0;
  let got = 0;
  await update.downloadAndInstall((event) => {
    if (event.event === "Started") total = event.data.contentLength ?? 0;
    else if (event.event === "Progress") {
      got += event.data.chunkLength;
      onProgress(total > 0 ? got / total : null);
    } else onProgress(1);
  });
  return true;
}

/** Restarts into the version just installed. */
export function restart(): Promise<void> {
  return relaunch();
}

/** A native item whose behaviour belongs to the OS: copy and paste, hide, quit. */
export type NativeMenuKind =
  | "About"
  | "Services"
  | "Hide"
  | "HideOthers"
  | "ShowAll"
  | "Quit"
  | "Undo"
  | "Redo"
  | "Cut"
  | "Copy"
  | "Paste"
  | "SelectAll"
  | "Minimize"
  | "Maximize"
  | "Fullscreen"
  | "CloseWindow"
  | "BringAllToFront";

/** An item that can be picked; `checked` present makes it a check item. */
export interface AppMenuItem {
  id: string;
  text: string;
  accelerator?: string;
  enabled?: boolean;
  checked?: boolean;
}

export interface AppMenuSubmenu {
  id: string;
  submenu: string;
  items: AppMenuEntry[];
  enabled?: boolean;
}

export type AppMenuEntry =
  "separator" | AppMenuItem | AppMenuSubmenu | { native: NativeMenuKind; text: string };

export interface AppMenuSection {
  text: string;
  items: AppMenuEntry[];
}

type Handle = MenuItem | CheckMenuItem | Submenu;

/** What the menu bar was last built from: its shape, and a handle for every item that can change. */
let built: { shape: string; handles: Map<string, Handle>; state: Map<string, string> } | null = null;
let queue: Promise<void> = Promise.resolve();

/** The spec without what changes in place, so a toggled check or a disabled item is not a rebuild. */
function shapeOf(sections: AppMenuSection[]): string {
  return JSON.stringify(sections, (key, value) =>
    key === "enabled" ? undefined : key === "checked" ? true : value,
  );
}

function stateOf(entry: AppMenuItem | AppMenuSubmenu): string {
  return `${entry.enabled !== false}|${"checked" in entry ? entry.checked : ""}`;
}

/**
 * Sets the macOS menu bar. Every text is the frontend's, already translated; a picked item comes
 * back as its id. The first section is the application menu, titled by the OS. A spec of the same
 * shape as the last one only updates `enabled` / `checked` on the items it already has, so
 * opening a dialog or changing the period does not rebuild the whole bar. Calls are serialised.
 */
export function setAppMenu(sections: AppMenuSection[], onPick: (id: string) => void): Promise<void> {
  // A failed build must not wedge every later one behind it.
  queue = queue.catch(() => {}).then(() => applyMenu(sections, onPick));
  return queue;
}

async function applyMenu(sections: AppMenuSection[], onPick: (id: string) => void): Promise<void> {
  const shape = shapeOf(sections);
  if (built && built.shape === shape) {
    const updates: Promise<void>[] = [];
    const walk = (entries: AppMenuEntry[]) => {
      for (const entry of entries) {
        if (entry === "separator" || "native" in entry) continue;
        const state = stateOf(entry);
        const handle = built!.handles.get(entry.id);
        if (handle && built!.state.get(entry.id) !== state) {
          built!.state.set(entry.id, state);
          updates.push(handle.setEnabled(entry.enabled !== false));
          if ("checked" in entry && handle instanceof CheckMenuItem)
            updates.push(handle.setChecked(!!entry.checked));
        }
        if ("submenu" in entry) walk(entry.items);
      }
    };
    sections.forEach((section) => walk(section.items));
    await Promise.all(updates);
    return;
  }

  const handles = new Map<string, Handle>();
  const state = new Map<string, string>();
  const build = async (entry: AppMenuEntry): Promise<Handle | PredefinedMenuItem> => {
    // eslint-disable-next-line lingui/no-unlocalized-strings -- a native item kind
    if (entry === "separator") return PredefinedMenuItem.new({ item: "Separator" });
    if ("native" in entry) {
      const kind = entry.native === "About" ? { About: null } : entry.native;
      return PredefinedMenuItem.new({ item: kind, text: entry.text });
    }
    let handle: Handle;
    if ("submenu" in entry) {
      const items = await Promise.all(entry.items.map(build));
      handle = await Submenu.new({
        id: entry.id,
        text: entry.submenu,
        enabled: entry.enabled !== false,
        items,
      });
    } else if ("checked" in entry) {
      const { checked, ...rest } = entry;
      handle = await CheckMenuItem.new({ ...rest, checked: !!checked, action: onPick });
    } else {
      handle = await MenuItem.new({ ...entry, action: onPick });
    }
    handles.set(entry.id, handle);
    state.set(entry.id, stateOf(entry));
    return handle;
  };
  const submenus = await Promise.all(
    sections.map(async (section) =>
      Submenu.new({ text: section.text, items: await Promise.all(section.items.map(build)) }),
    ),
  );
  const menu = await Menu.new({ items: submenus });
  await menu.setAsAppMenu();
  built = { shape, handles, state };
}

/** Returns today's local calendar date without timezone shifting. */
export function today(): DateString {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`;
}
