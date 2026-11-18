import { invoke } from "@tauri-apps/api/core";

// Daily goal commands (S4). Failures reject with the backend's Spanish message.

export type GoalKind = "minutes" | "texts";

/** The active profile's daily goal. */
export interface DailyGoal {
  kind: GoalKind;
  amount: number;
}

/** The goal (`null` = off), today's whole minutes or texts, whether it is met and the streak in days. */
export interface GoalStatus {
  goal: DailyGoal | null;
  today: number;
  met: boolean;
  streak: number;
}

/** `tzOffsetMin`: minutes east of UTC (`-new Date().getTimezoneOffset()`). */
export const goalStatus = (tzOffsetMin: number) => invoke<GoalStatus>("goal_status", { tzOffsetMin });

/** Sets the active profile's goal; `"off"` goes without amount. */
export const goalSet = (kind: GoalKind | "off", amount: number | null, tzOffsetMin: number) =>
  invoke<GoalStatus>("goal_set", { kind, amount, tzOffsetMin });

/** The goal commands, as an object so the settings store can run against a fake backend. */
export interface GoalsApi {
  status(tzOffsetMin: number): Promise<GoalStatus>;
  set(kind: GoalKind | "off", amount: number | null, tzOffsetMin: number): Promise<GoalStatus>;
}

export const goalsApi: GoalsApi = { status: goalStatus, set: goalSet };
