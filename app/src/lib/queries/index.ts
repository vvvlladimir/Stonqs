/**
 * The data layer: one place where a query key is spelled and one place that says what a write
 * invalidates. A screen calls a hook, never `useQuery` with a key. A hook with a nullable id or an
 * absent range stays disabled itself, so callers carry no `enabled` and no `!`.
 */

export { keys } from "./keys";
export { affects, useInvalidate } from "./invalidation";
export * from "./app";
export * from "./plugins";
export * from "./ai";
export * from "./ledger";
export * from "./plans";
export * from "./securities";
export * from "./reports";
export * from "./taxonomy";
