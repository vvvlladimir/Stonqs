/** The only frontend code that imports Tauri APIs; screens import `api` from here. */

import { ledgerApi } from "./ledger";
import { securitiesApi } from "./securities";
import { plansApi } from "./plans";
import { reportsApi } from "./reports";
import { taxonomyApi } from "./taxonomy";
import { importApi } from "./import";
import { marketApi } from "./market";
import { pluginsApi } from "./plugins";
import { aiApi } from "./ai";
import { appApi } from "./app";

export { ApiError, today, type Source } from "./core";
export * from "./events";
export * from "./menu";
export * from "./updates";

export const api = {
  ...ledgerApi,
  ...securitiesApi,
  ...plansApi,
  ...reportsApi,
  ...taxonomyApi,
  ...importApi,
  ...marketApi,
  ...pluginsApi,
  ...aiApi,
  ...appApi,
};
