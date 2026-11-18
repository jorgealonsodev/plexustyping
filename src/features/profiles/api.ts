import { invoke } from "@tauri-apps/api/core";

// Profile commands (S3). Failures reject with the backend's Spanish message.

/** A local profile: its history, statistics, layout, language and theme (`null` = none chosen yet). */
export interface Profile {
  id: number;
  name: string;
  language: string;
  layout: string;
  theme: string | null;
  createdAtMs: number;
}

export interface ProfilesList {
  active: number;
  profiles: Profile[];
}

/** Fields to change; a missing one is left as is, and an empty `theme` clears it. */
export interface ProfileFields {
  name?: string;
  language?: string;
  layout?: string;
  theme?: string;
}

export const profilesList = () => invoke<ProfilesList>("profiles_list");

/** Creates a profile; the active one does not change. */
export const profileCreate = (name: string, language: string, layout: string) =>
  invoke<Profile>("profile_create", { name, language, layout });

export const profileUpdate = (id: number, fields: ProfileFields) =>
  invoke<Profile>("profile_update", {
    id,
    name: fields.name ?? null,
    language: fields.language ?? null,
    layout: fields.layout ?? null,
    theme: fields.theme ?? null,
  });

/** Deletes a profile and its data; `active` is the profile active afterwards. */
export const profileDelete = (id: number) => invoke<{ active: Profile }>("profile_delete", { id });

export const profileSwitch = (id: number) => invoke<Profile>("profile_switch", { id });

/** The profile commands, as an object so the profile logic can run against a fake backend. */
export interface ProfilesApi {
  list(): Promise<ProfilesList>;
  create(name: string, language: string, layout: string): Promise<Profile>;
  update(id: number, fields: ProfileFields): Promise<Profile>;
  remove(id: number): Promise<{ active: Profile }>;
  switchTo(id: number): Promise<Profile>;
}

export const profilesApi: ProfilesApi = {
  list: profilesList,
  create: profileCreate,
  update: profileUpdate,
  remove: profileDelete,
  switchTo: profileSwitch,
};
