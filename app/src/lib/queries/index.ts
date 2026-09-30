/**
 * The data layer: keys spelled once, invalidation named once, a hook per query. A hook with a
 * nullable id stays disabled itself, so callers carry no `enabled` and no `!`.
 */

export { keys } from "./keys";
export { affects, useInvalidate } from "./invalidation";
export * from "./app";
export * from "./plugins";
export * from "./ai";
export * from "./ledger";
export * from "./plans";
export * from "./networth";
export * from "./securities";
export * from "./reports";
export * from "./taxonomy";
