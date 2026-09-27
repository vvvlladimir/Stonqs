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

/**
 * Moves that arrived as two unrelated rows. Read on demand — the import screen asks after a
 * write — rather than on every ledger render: it is a whole-portfolio scan, and nothing on the
 * screen is wrong while the answer is missing.
 */
export function useTransferSuggestions(enabled: boolean) {
  return useQuery({
    queryKey: keys.transferSuggestions(),
    queryFn: api.transferSuggestions,
    enabled,
  });
}
