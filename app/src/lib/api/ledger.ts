/** Portfolio, accounts, the scope picker and the ledger itself. */

import type {
  Account,
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
  AppStatus,
  DashboardData,
  DateString,
  Portfolio,
  QuantityGap,
  PortfolioInput,
  SetupInput,
} from "../types";
import { call, type Source } from "./core";

export const ledgerApi = {
  appStatus: () => call<AppStatus>("app_status"),
  dashboardSummary: (date: DateString, source?: Source) =>
    call<DashboardData>("dashboard_summary", { date, source: source ?? null }),

  portfolioGet: () => call<Portfolio>("portfolio_get"),
  portfolioGaps: () => call<QuantityGap[]>("portfolio_gaps"),
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

  transactionsList: (filter: TransactionFilter) => call<TransactionsData>("transactions_list", { filter }),
  /** `format` is a plugin writer's key; null saves the app's own file. */
  transactionsExportSave: (filter: TransactionFilter, path: string, format: string | null = null) =>
    call<void>("transactions_export_save", { filter, path, format }),
  transactionSave: (input: TransactionInput) => call<Transaction>("transaction_save", { input }),
  transactionDelete: (id: string) => call<void>("transaction_delete", { id }),
  /** Moves between two of the user's own accounts that arrived as two unrelated rows. */
  transferSuggestions: () => call<TransferSuggestion[]>("transfer_suggestions"),
  transferLink: (ids: [string, string]) => call<void>("transfer_link", { ids }),
};
