/** Realised gains, dividends and charges. */

import type { DateString, MoneyString } from "./primitives";
import type { DividendFrequency } from "./securities";
import type { TransactionKind } from "./transactions";
export interface RealizedSummary {
  disposals: number;
  proceeds_base: MoneyString;
  cost_base: MoneyString;
  fees_base: MoneyString;
  taxes_base: MoneyString;
  gain_base: MoneyString;
  /** The exchange rate's share of `gain_base`, each disposal at its own rate. */
  currency_gain_base: MoneyString;
}

export interface DividendSummary {
  payments: number;
  gross_base: MoneyString;
  taxes_base: MoneyString;
  fees_base: MoneyString;
  net_base: MoneyString;
}

export interface YearGains extends RealizedSummary {
  year: number;
  /** Result against the cost sold; null when there was no cost to divide by. */
  return_on_cost: MoneyString | null;
}

export interface SecurityGains extends RealizedSummary {
  security_id: string;
  symbol: string;
  name: string;
  return_on_cost: MoneyString | null;
}

/** One disposal as the broker made it — the ledger behind the yearly rollup. */
export interface DisposalRow {
  date: DateString;
  security_id: string;
  /** Sale or outbound delivery: both realize a result. */
  kind: TransactionKind;
  quantity: MoneyString;
  proceeds_base: MoneyString;
  fees_base: MoneyString;
  taxes_base: MoneyString;
  cost_base: MoneyString;
  gain_base: MoneyString;
  /** The exchange rate's share of `gain_base`, at this disposal's own rate. */
  currency_gain_base: MoneyString;
  symbol: string;
  name: string;
  return_on_cost: MoneyString | null;
}

export interface YearDividends extends DividendSummary {
  year: number;
}

export interface SecurityDividends extends DividendSummary {
  security_id: string;
  symbol: string;
  name: string;
  /** Lifetime dividends over the open position's cost — not a figure of the window. */
  yield_on_cost: MoneyString | null;
  /** Read off the whole payment history, like `yield_on_cost` and unlike the summary. */
  frequency: DividendFrequency;
  last_payment: DateString | null;
  payments_total: number;
  trailing_year_base: MoneyString;
  annual_yield_on_cost: MoneyString | null;
}

/** One dividend payment, with the broker's own currency kept beside the base one. */
export interface DividendPaymentRow {
  date: DateString;
  account_id: string;
  security_id: string | null;
  kind: TransactionKind;
  gross_base: MoneyString;
  taxes_base: MoneyString;
  fees_base: MoneyString;
  net_base: MoneyString;
  currency: string;
  gross_in_currency: MoneyString;
  symbol: string;
  name: string;
  account: string;
}

/** One standalone fee or tax; refunds are negative. */
export interface ChargeRow {
  date: DateString;
  account_id: string;
  security_id: string | null;
  kind: TransactionKind;
  amount_base: MoneyString;
  currency: string;
  amount_in_currency: MoneyString;
  symbol: string;
  name: string;
  account: string;
}

export interface ReportsData {
  from: DateString;
  to: DateString;
  /** The equal-length window immediately before, for the change tiles. */
  previous_from: DateString;
  previous_to: DateString;
  base_currency: string;

  gains_by_year: YearGains[];
  gains_by_security: SecurityGains[];
  gains_total: RealizedSummary;
  gains_previous: RealizedSummary;
  /** Movement against the previous window, computed by the host. */
  gains_change_base: MoneyString;
  /** Result of the whole window against the cost it came from; null when there is none. */
  gains_return_on_cost: MoneyString | null;
  disposals: DisposalRow[];

  dividends_by_year: YearDividends[];
  dividends_by_security: SecurityDividends[];
  dividends_total: DividendSummary;
  dividends_previous: DividendSummary;
  dividends_change_base: MoneyString;
  payments: DividendPaymentRow[];

  /** Fees and taxes recorded as separate transactions. */
  charges_by_year: YearCharges[];
  charges_by_kind: KindCharges[];
  charges_by_account: AccountCharges[];
  charges_total: ChargeSummary;
  charges_previous: ChargeSummary;
  charges_change_base: MoneyString;
  /** Fees plus taxes over the window. */
  charges_total_base: MoneyString;
  charge_rows: ChargeRow[];
}

/** Aggregated charges; refunds are negative. */
export interface ChargeSummary {
  count: number;
  fees_base: MoneyString;
  taxes_base: MoneyString;
}

export interface YearCharges extends ChargeSummary {
  year: number;
  total_base: MoneyString;
}

export interface KindCharges extends ChargeSummary {
  kind: TransactionKind;
  total_base: MoneyString;
}

export interface AccountCharges extends ChargeSummary {
  account_id: string;
  account: string;
  total_base: MoneyString;
}
