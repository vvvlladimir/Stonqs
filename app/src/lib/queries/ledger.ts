/** Portfolio, accounts, scope and the transaction ledger. */

import { useQuery } from "@tanstack/react-query";
import { api } from "../api";
import type { TransactionFilter } from "../types";
import { keys } from "./keys";

export function usePortfolio() {
  return useQuery({ queryKey: keys.portfolio(), queryFn: api.portfolioGet });
}

/** Sales of shares the ledger never received, over the whole portfolio. */
export function useLedgerGaps() {
  return useQuery({ queryKey: keys.ledgerGaps(), queryFn: api.portfolioGaps });
}

export function useAccounts() {
  return useQuery({ queryKey: keys.accounts(), queryFn: api.accountsList });
}

export function useAccountGroups() {
  return useQuery({ queryKey: keys.accountGroups(), queryFn: api.accountGroupsList });
}

export function useAccountsTotal() {
  return useQuery({ queryKey: keys.accountsTotal(), queryFn: api.accountsTotal });
}

export function useScope() {
  return useQuery({ queryKey: keys.scope(), queryFn: api.scopeGet });
}

export function useTransactions(filter: TransactionFilter) {
  return useQuery({
    queryKey: keys.transactions(filter),
    queryFn: () => api.transactionsList(filter),
  });
}

/** On demand only: a whole-portfolio scan. */
export function useTransferSuggestions(enabled: boolean) {
  return useQuery({
    queryKey: keys.transferSuggestions(),
    queryFn: api.transferSuggestions,
    enabled,
  });
}
