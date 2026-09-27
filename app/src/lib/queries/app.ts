/** App status, profiles, settings and saved import layouts. */

import { useQuery } from "@tanstack/react-query";
import { api } from "../api";
import { keys } from "./keys";

export function useStatus() {
  return useQuery({ queryKey: keys.status(), queryFn: api.appStatus });
}

export function useProfiles() {
  return useQuery({ queryKey: keys.profiles(), queryFn: api.profilesList });
}

export function useSettings() {
  return useQuery({ queryKey: keys.settings(), queryFn: api.settingsGet });
}

export function useImportTemplates() {
  return useQuery({ queryKey: keys.importTemplates(), queryFn: api.importTemplates });
}
