import type { AccountGroupInput, AccountInput } from "../../lib/types";

export const EMPTY_ACCOUNT: AccountInput = {
  id: null,
  name: "",
  currency: "EUR",
  kind: "DEPOSIT",
  reference_account_id: null,
  is_active: true,
  opened_at: null,
};

export const EMPTY_GROUP: AccountGroupInput = { id: null, name: "", account_ids: [] };
