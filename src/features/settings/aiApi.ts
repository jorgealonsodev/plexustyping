import { invoke } from "@tauri-apps/api/core";
import type { AiSettings, ProviderId, ProviderInfo, UsageRow } from "./ai";

// AI commands. Keys only travel to `ai_key_save`; no command ever returns one.

export const aiSettingsGet = () => invoke<AiSettings>("ai_settings_get");

export const aiSettingsSet = (settings: AiSettings) => invoke<AiSettings>("ai_settings_set", { settings });

export const aiProviders = () => invoke<ProviderInfo[]>("ai_providers");

export const aiKeyStatus = (provider: ProviderId) => invoke<boolean>("ai_key_status", { provider });

export const aiKeySave = (provider: ProviderId, key: string) => invoke<void>("ai_key_save", { provider, key });

export const aiKeyDelete = (provider: ProviderId) => invoke<void>("ai_key_delete", { provider });

export const aiModels = (provider: ProviderId) => invoke<string[]>("ai_models", { provider });

export const aiTest = (provider: ProviderId, model: string) => invoke<void>("ai_test", { provider, model });

export const aiUsage = (day: string) => invoke<UsageRow[]>("ai_usage", { day });
