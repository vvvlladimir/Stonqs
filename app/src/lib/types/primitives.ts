/** The scalars every other file is written in, and the change notification the host sends. */

/** Monetary decimal serialized as a string to preserve exactness across IPC. */
export type MoneyString = string;

/** Date in `YYYY-MM-DD` format. */
export type DateString = string;

/** What a host write touched, as emitted by `events.rs`. */
export type DataChangeKind =
  | "accounts"
  | "ai_chats"
  | "alerts"
  | "goals"
  | "plans"
  | "portfolio"
  | "quotes"
  | "scope"
  | "securities"
  | "targets"
  | "taxonomies"
  | "transactions"
  | "watchlists";

/** Payload of `data:changed`; `scope` is the wire name, not the data scope. */
export interface DataChanged {
  scope: DataChangeKind;
}

/** Structured host error; callers branch on `code`, not message text. */
export type UiError =
  | { code: "storage"; message: string }
  | { code: "network"; message: string }
  | { code: "not_found"; message: string }
  | { code: "invalid"; message: string }
  | { code: "missing_market_data"; kind: string; key: string; date: DateString; message: string }
  | { code: "math"; message: string }
  /** A provider rejected the saved key, or none is saved: the fix is a new key, not a retry. */
  | { code: "auth"; message: string }
  /** A provider is throttling; `retry_after` is the seconds it asked for, when it said. */
  | { code: "rate_limit"; message: string; retry_after: number | null }
  /** A provider answered with an error of its own; `message` is its line, shown after ours. */
  | { code: "provider"; message: string }
  /** The model declined to answer; what it did say stays in the chat. */
  | { code: "refused"; message: string }
  /** The answer reached the model's output limit and stops mid-way. */
  | { code: "truncated"; message: string }
  | { code: "locked"; message: string }
  | { code: "password_required"; message: string }
  | { code: "wrong_password"; message: string }
  | { code: "busy"; message: string }
  /** A plugin's file reader recognised the file and needs the password it is sealed with. */
  | { code: "file_protected"; message: string }
  /** A plugin's file reader failed over a file it claimed: `plugin` is which one. */
  | { code: "reader"; plugin: string; message: string }
  | { code: "internal"; message: string };

// Positions
