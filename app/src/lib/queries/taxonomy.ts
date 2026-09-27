/** Classification trees, allocation, targets and rebalancing. */

import { useQuery } from "@tanstack/react-query";
import { api, type Source } from "../api";
import type { DateString } from "../types";
import { keys } from "./keys";

/** What grouping a tree by one attribute would do, before it is written. */
export function useTaxonomyGrouping(attributeId: string | null, into: string | null) {
  return useQuery({
    queryKey: keys.taxonomyGrouping(attributeId ?? undefined, into ?? undefined),
    queryFn: () => api.taxonomyGroupPreview(attributeId!, into),
    enabled: attributeId !== null,
  });
}

export function useTaxonomies() {
  return useQuery({ queryKey: keys.taxonomies(), queryFn: api.taxonomiesList });
}

/** `taxonomyId` null means the security breakdown, which needs no tree. */
export function useAllocation(cut: string, taxonomyId: string | null, date: DateString, source?: Source) {
  return useQuery({
    queryKey: keys.allocation(cut, taxonomyId, date, source),
    queryFn: () => api.allocation(cut, taxonomyId, date, source),
    enabled: cut !== "taxonomy" || taxonomyId !== null,
  });
}

export function useAllocationMembers(
  taxonomyId: string | null,
  nodeId: string | null,
  date: DateString,
  source?: Source,
) {
  return useQuery({
    queryKey: keys.allocationMembers(taxonomyId ?? undefined, nodeId, date, source),
    queryFn: () => api.allocationMembers(taxonomyId!, nodeId, date, source),
    enabled: taxonomyId !== null,
  });
}

export function useAllocationTree(taxonomyId: string | null, date: DateString, source?: Source) {
  return useQuery({
    queryKey: keys.allocationTree(taxonomyId ?? undefined, date, source),
    queryFn: () => api.allocationTree(taxonomyId!, date, source),
    enabled: taxonomyId !== null,
  });
}

export function useTargets() {
  return useQuery({ queryKey: keys.targets(), queryFn: api.targetsList });
}

export function useRebalance(
  targetId: string | null,
  date: DateString,
  cash: string | null = null,
  allowSell = true,
  source?: Source,
) {
  return useQuery({
    queryKey: keys.rebalance(targetId ?? undefined, date, cash, allowSell, source),
    queryFn: () => api.rebalancePlan(targetId!, date, cash, allowSell, source),
    enabled: targetId !== null,
  });
}
