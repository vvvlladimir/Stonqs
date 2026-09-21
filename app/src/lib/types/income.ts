/** Income, rolled up every way the screen offers. */

import type { DateString, MoneyString } from "./primitives";
import type { TransactionKind } from "./transactions";
/** Aggregated income; paid interest has negative `net_base`. */
export interface IncomeSummary {
  events: number;
  gross_base: MoneyString;
  taxes_base: MoneyString;
  fees_base: MoneyString;
  net_base: MoneyString;
}

/** One income event; account interest has no security ID. */
export interface IncomeEvent {
  date: DateString;
  account_id: string;
  security_id: string | null;
  kind: TransactionKind;
  gross_base: MoneyString;
  taxes_base: MoneyString;
  fees_base: MoneyString;
  net_base: MoneyString;
  /** Transaction currency and broker-reported gross amount. */
  currency: string;
  gross_in_currency: MoneyString;
  symbol: string;
  name: string;
}

export interface MonthIncome extends IncomeSummary {
  year: number;
  month: number;
}

export interface SecurityIncome extends IncomeSummary {
  security_id: string;
  symbol: string;
  name: string;
}

export interface KindIncome extends IncomeSummary {
  kind: TransactionKind;
}

/** One month index with every year folded in: the calendar's bottom row. */
export interface MonthOfYearIncome extends IncomeSummary {
  month: number;
}

/** One kind inside one year: the composition of a single year bar. */
export interface YearKindIncome extends IncomeSummary {
  year: number;
  kind: TransactionKind;
}

export interface IncomeData {
  from: DateString;
  to: DateString;
  base_currency: string;
  events: IncomeEvent[];
  by_month: MonthIncome[];
  by_month_of_year: MonthOfYearIncome[];
  by_security: SecurityIncome[];
  by_kind: KindIncome[];
  total: IncomeSummary;
  /** Total for the immediately preceding equal-length window. */
  previous_total: IncomeSummary;
  previous_from: DateString;
  previous_to: DateString;
  /** Core-computed difference between current and previous net income. */
  change_base: MoneyString;
  /** Historical yearly income for year-over-year bars. */
  by_year: YearIncome[];
  /** The same yearly income split by kind. */
  by_year_kind: YearKindIncome[];
  /** Kind the report was narrowed to; null means every kind. */
  kind: TransactionKind | null;
}

export interface YearIncome extends IncomeSummary {
  year: number;
}

// CSV import

/** One tree node with the income of everything below it. */
export interface IncomeNode {
  /** Node id, or `UNCLASSIFIED_KEY`. */
  key: string;
  label: string;
  summary: IncomeSummary;
  /** Share of the tree's net income; interest charged can make it negative. */
  weight: string;
  children: IncomeNode[];
}

/** The period's income read through one classification tree. */
export interface TaxonomyIncomeData {
  taxonomy_id: string;
  base_currency: string;
  /** Everything the tree saw; an excluded subject is not in it. */
  total: IncomeSummary;
  nodes: IncomeNode[];
}
