import { invoke } from "@tauri-apps/api/core";
import type { AiSettings, ChatgptSignOut, ChatgptStatus, ModelOption, ProviderId, ProviderInfo, UsageRow } from "./ai";

// AI commands. Keys only travel to `ai_key_save`; no command ever returns one,
// nor a ChatGPT token.

export const aiSettingsGet = () => invoke<AiSettings>("ai_settings_get");

export const aiSettingsSet = (settings: AiSettings) => invoke<AiSettings>("ai_settings_set", { settings });

export const aiProviders = () => invoke<ProviderInfo[]>("ai_providers");

export const aiKeyStatus = (provider: ProviderId) => invoke<boolean>("ai_key_status", { provider });

export const aiKeySave = (provider: ProviderId, key: string) => invoke<void>("ai_key_save", { provider, key });

export const aiKeyDelete = (provider: ProviderId) => invoke<void>("ai_key_delete", { provider });

export const aiModels = (provider: ProviderId) => invoke<string[]>("ai_models", { provider });

/** Listed models with the name to show: ChatGPT's display names, the id for the others. */
export const aiModelOptions = (provider: ProviderId): Promise<ModelOption[]> =>
  provider === "chatgpt"
    ? invoke<ModelOption[]>("ai_chatgpt_models")
    : aiModels(provider).then((ids) => ids.map((id) => ({ id, name: id })));

export const aiChatgptStatus = () => invoke<ChatgptStatus>("ai_chatgpt_status");

/** Opens the system browser; resolves once it comes back (or on cancel or timeout). */
export const aiChatgptSignIn = () => invoke<ChatgptStatus>("ai_chatgpt_sign_in");

export const aiChatgptCancel = () => invoke<void>("ai_chatgpt_cancel");

export const aiChatgptSignOut = () => invoke<ChatgptSignOut>("ai_chatgpt_sign_out");

/** Opens https://chatgpt.com/settings/usage in the system browser. */
export const aiChatgptOpenUsage = () => invoke<void>("ai_chatgpt_open_usage");

export const aiTest = (provider: ProviderId, model: string) => invoke<void>("ai_test", { provider, model });

export const aiUsage = (day: string) => invoke<UsageRow[]>("ai_usage", { day });
