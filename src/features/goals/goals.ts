import { localDay } from "../practice/prefetch";
import type { DailyGoal, GoalKind, GoalStatus } from "./api";
import { t } from "./messages";

// Daily goal and streak (S1, S5): choices, header text and the one-time notice.

/** Allowed daily goals (S1), as the backend accepts them. */
export const MINUTE_GOALS: readonly number[] = [5, 10, 15, 20, 30, 45, 60, 90, 120];
export const TEXT_GOALS: readonly number[] = [1, 3, 5, 10, 15, 20, 30];

export type GoalChoice = GoalKind | "off";

/** The goal kinds; labels follow the interface language. */
export const GOAL_KINDS: readonly { readonly id: GoalChoice; readonly label: string }[] = (
  ["off", "minutes", "texts"] as const
).map((id) => ({
  id,
  get label() {
    return t(id);
  },
}));

/** Amount picked when a kind is chosen without a fitting one. */
const DEFAULT_AMOUNT: Record<GoalKind, number> = { minutes: 20, texts: 5 };

/** Shown once on the result screen of the session that meets the goal (S5). */
export const goalMetNotice = () => t("metNotice");

export const amountsFor = (kind: GoalChoice): readonly number[] =>
  kind === "minutes" ? MINUTE_GOALS : kind === "texts" ? TEXT_GOALS : [];

/** The goal to set for `kind`: `amount` (else the current goal's) when allowed for it, else its default. */
export function chooseGoal(
  kind: GoalChoice,
  current: DailyGoal | null,
  amount: number | null = current?.kind === kind ? current.amount : null,
): { kind: GoalChoice; amount: number | null } {
  if (kind === "off") return { kind, amount: null };
  return { kind, amount: amount !== null && amountsFor(kind).includes(amount) ? amount : DEFAULT_AMOUNT[kind] };
}

/** "Hoy 12/20 min · racha 3", "Hoy 2/5 textos", "Meta cumplida · racha 4"; `null` without a goal. */
export function goalHeaderText(status: GoalStatus | null): string | null {
  const goal = status?.goal;
  if (!status || !goal) return null;
  const streak = status.streak > 0 ? t("streak", { days: status.streak }) : "";
  if (status.met) return t("met", { streak });
  const unit = goal.kind === "minutes" ? t("minutesUnit") : t("textsUnit");
  return t("today", { done: status.today, goal: goal.amount, unit, streak });
}

/** A goal status and the local day (`YYYY-MM-DD`) it was read on. */
export interface GoalSnapshot {
  status: GoalStatus;
  day: string;
}

const sameGoal = (a: DailyGoal | null, b: DailyGoal | null) =>
  a !== null && b !== null && a.kind === b.kind && a.amount === b.amount;

/** Whether the session between `before` and `after` met the goal: met now, not met before on the same day. */
export function crossedGoal(before: GoalSnapshot | null, after: GoalSnapshot): boolean {
  if (!before || !after.status.met || !sameGoal(before.status.goal, after.status.goal)) return false;
  return before.day !== after.day || !before.status.met;
}

/** Minutes east of UTC, as the goal commands take them. */
export const tzOffsetMin = (now: Date = new Date()) => -now.getTimezoneOffset();

/** The local `YYYY-MM-DD` a snapshot is read on. */
export { localDay };
