import { afterEach, describe, expect, it } from "vitest";
import { ui } from "../../i18n/i18n.svelte";
import type { SessionSummary } from "./api";
import {
  CHART_LIMIT,
  FILTERS,
  chartCaption,
  chartGeometry,
  exportNotice,
  formatDate,
  formatSummary,
  isHistoryShortcut,
  modeArg,
  requestGuard,
  summarize,
  tableRow,
} from "./history";

const at = (y: number, mo: number, d: number, h = 0, mi = 0) => new Date(y, mo - 1, d, h, mi).getTime();

function session(over: Partial<SessionSummary> = {}): SessionSummary {
  return {
    id: 1,
    startedAtMs: at(2026, 11, 17, 9, 30),
    mode: "prose",
    codeLanguage: null,
    language: "es",
    layout: "es-qwerty",
    durationMs: 65_400,
    grossWpm: 45,
    netWpm: 41.5,
    accuracy: 0.905,
    keystrokes: 230,
    bpm: null,
    ...over,
  };
}

const key = (over: Partial<KeyboardEvent>) =>
  ({ key: "h", ctrlKey: true, altKey: false, metaKey: false, shiftKey: false, repeat: false, ...over }) as KeyboardEvent;

describe("history filter", () => {
  it("maps the filter to the backend mode argument", () => {
    expect([modeArg("all"), modeArg("prose"), modeArg("words"), modeArg("code")]).toEqual([null, "prose", "words", "code"]);
  });

  it("lists the filters in the header order", () => {
    expect(FILTERS.map((f) => f.label)).toEqual(["Todo", "Prosa", "Palabras", "Código"]);
  });
});

describe("history shortcut", () => {
  it("is plain Ctrl+H, never a held key", () => {
    expect(isHistoryShortcut(key({}))).toBe(true);
    expect(isHistoryShortcut(key({ key: "H" }))).toBe(true);
    expect(isHistoryShortcut(key({ ctrlKey: false }))).toBe(false);
    expect(isHistoryShortcut(key({ shiftKey: true }))).toBe(false);
    expect(isHistoryShortcut(key({ altKey: true }))).toBe(false);
    expect(isHistoryShortcut(key({ metaKey: true }))).toBe(false);
    expect(isHistoryShortcut(key({ repeat: true }))).toBe(false);
    expect(isHistoryShortcut(key({ key: "," }))).toBe(false);
  });
});

describe("history summary", () => {
  it("is null without sessions", () => {
    expect(summarize([])).toBeNull();
  });

  it("counts sessions and averages net WPM and accuracy, keeping the best net WPM", () => {
    const s = summarize([session({ netWpm: 40, accuracy: 0.9 }), session({ netWpm: 60, accuracy: 1 }), session({ netWpm: 50, accuracy: 0.95 })]);
    expect(s).toEqual({ count: 3, meanNetWpm: 50, bestNetWpm: 60, meanAccuracy: expect.closeTo(0.95) });
  });

  it("formats the summary in Spanish", () => {
    expect(formatSummary({ count: 3, meanNetWpm: 41.5, bestNetWpm: 60.4, meanAccuracy: 0.905 })).toEqual([
      { label: "Sesiones", value: "3" },
      { label: "PPM netas (media)", value: "42" },
      { label: "PPM netas (mejor)", value: "60" },
      { label: "Precisión media", value: "90,5 %" },
    ]);
  });
});

describe("history table", () => {
  it("formats the local date as dd/mm/aaaa hh:mm", () => {
    expect(formatDate(at(2026, 1, 5, 7, 3))).toBe("05/01/2026 07:03");
  });

  it("formats a prose row; BPM is empty when null", () => {
    expect(tableRow(session(), (id) => id)).toEqual({
      id: 1,
      date: "17/11/2026 09:30",
      mode: "Prosa",
      netWpm: "42",
      accuracy: "90,5 %",
      time: "1:05",
      bpm: "",
    });
  });

  it("formats a code row with its language name and BPM", () => {
    const row = tableRow(session({ id: 7, mode: "code", codeLanguage: "python", bpm: 90 }), (id) =>
      id === "python" ? "Python" : id,
    );
    expect(row.mode).toBe("Código · Python");
    expect(row.bpm).toBe("90");
    expect(row.id).toBe(7);
  });

  it("shows a words row as Palabras", () => {
    expect(tableRow(session({ mode: "words" }), (id) => id).mode).toBe("Palabras");
    expect(tableRow(session({ mode: "prose" }), (id) => id).mode).toBe("Prosa");
  });

  it("shows a code row without a language as plain code", () => {
    expect(tableRow(session({ mode: "code", codeLanguage: null }), (id) => id).mode).toBe("Código");
  });
});

describe("history chart", () => {
  const box = { width: 600, height: 200, left: 40, right: 40, top: 10, bottom: 20 };

  it("has no points without sessions", () => {
    const g = chartGeometry([], box);
    expect(g.wpm).toEqual([]);
    expect(g.accuracy).toEqual([]);
    expect(g.wpmLine).toBe("");
    expect(g.accuracyLine).toBe("");
  });

  it("centres a single session horizontally", () => {
    const g = chartGeometry([session({ netWpm: 25, accuracy: 0.5 })], box);
    expect(g.wpm).toHaveLength(1);
    expect(g.wpm[0].x).toBe(300);
    expect(g.accuracy[0].x).toBe(300);
    // Scale 0..30 WPM (rounded up to a multiple of 10), plot area y 10..180.
    expect(g.wpmMax).toBe(30);
    expect(g.wpm[0].y).toBeCloseTo(180 - (25 / 30) * 170);
    expect(g.accuracy[0].y).toBeCloseTo(95);
  });

  it("draws the last sessions oldest left, newest right, on 0..max WPM and 0..100 %", () => {
    // Newest first, as the backend returns them.
    const sessions = [session({ id: 3, netWpm: 60, accuracy: 1 }), session({ id: 2, netWpm: 30, accuracy: 0.5 }), session({ id: 1, netWpm: 0, accuracy: 0 })];
    const g = chartGeometry(sessions, box);
    expect(g.wpmMax).toBe(60);
    expect(g.wpm.map((p) => p.x)).toEqual([40, 300, 560]);
    expect(g.wpm.map((p) => p.y)).toEqual([180, 95, 10]);
    expect(g.accuracy.map((p) => p.y)).toEqual([180, 95, 10]);
    expect(g.wpm.map((p) => p.id)).toEqual([1, 2, 3]);
    expect(g.wpmLine).toBe("40,180 300,95 560,10");
    expect(g.accuracyLine).toBe("40,180 300,95 560,10");
  });

  it("keeps only the newest 50 sessions", () => {
    const sessions = Array.from({ length: 60 }, (_, i) => session({ id: 60 - i, netWpm: 60 - i }));
    const g = chartGeometry(sessions, box);
    expect(CHART_LIMIT).toBe(50);
    expect(g.wpm).toHaveLength(50);
    expect(g.wpm[0].id).toBe(11);
    expect(g.wpm[49].id).toBe(60);
    expect(g.wpm[0].x).toBe(40);
    expect(g.wpm[49].x).toBe(560);
  });

  it("uses at least a 0..10 WPM scale and gives gridline ticks", () => {
    const g = chartGeometry([session({ netWpm: 0 })], box);
    expect(g.wpmMax).toBe(10);
    expect(g.ticks.map((t) => t.y)).toEqual([180, 95, 10]);
    expect(g.ticks.map((t) => t.wpm)).toEqual(["0", "5", "10"]);
    expect(g.ticks.map((t) => t.accuracy)).toEqual(["0 %", "50 %", "100 %"]);
  });
});

describe("export notice", () => {
  it("reports the path and row count", () => {
    expect(exportNotice({ path: "/home/u/Descargas/plexustyping-sesiones-20261117.csv", rows: 3 })).toBe(
      "Exportado: /home/u/Descargas/plexustyping-sesiones-20261117.csv (3 sesiones)",
    );
  });

  it("uses the singular for one session", () => {
    expect(exportNotice({ path: "/tmp/a.csv", rows: 1 })).toBe("Exportado: /tmp/a.csv (1 sesión)");
  });
});

describe("chart caption", () => {
  it("uses the singular for one session and the plural for several", () => {
    expect(chartCaption(1)).toBe("Última sesión");
    expect(chartCaption(2)).toBe("Últimas 2 sesiones");
    expect(chartCaption(50)).toBe("Últimas 50 sesiones");
  });
});

describe("request guard", () => {
  it("accepts only the newest request", () => {
    const g = requestGuard();
    const a = g.next();
    expect(g.isCurrent(a)).toBe(true);
    const b = g.next();
    expect(g.isCurrent(a)).toBe(false);
    expect(g.isCurrent(b)).toBe(true);
  });

  it("drops an in-flight request when invalidated (filter change, close and reopen)", () => {
    const g = requestGuard();
    const a = g.next();
    g.invalidate();
    expect(g.isCurrent(a)).toBe(false);
    const b = g.next();
    expect(g.isCurrent(b)).toBe(true);
  });
});

describe("history in the interface language (ui-i18n S1)", () => {
  afterEach(() => ui.set("es"));

  it("follows the interface language for filters, summary, modes and notices", () => {
    ui.set("en");
    expect(FILTERS.map((f) => f.label)).toEqual(["All", "Prose", "Words", "Code"]);
    expect(formatSummary({ count: 3, meanNetWpm: 41.5, bestNetWpm: 60.4, meanAccuracy: 0.905 }).map((i) => i.label)).toEqual([
      "Sessions",
      "Net WPM (average)",
      "Net WPM (best)",
      "Average accuracy",
    ]);
    expect(tableRow(session({ mode: "words" }), (id) => id).mode).toBe("Words");
    expect(tableRow(session({ mode: "code", codeLanguage: "python" }), () => "Python").mode).toBe("Code · Python");
    expect(exportNotice({ path: "/tmp/a.csv", rows: 1 })).toBe("Exported: /tmp/a.csv (1 session)");
    expect(exportNotice({ path: "/tmp/a.csv", rows: 3 })).toBe("Exported: /tmp/a.csv (3 sessions)");
  });

  it("uses the interface language's plural in the chart caption", () => {
    ui.set("en");
    expect(chartCaption(1)).toBe("Last session");
    expect(chartCaption(2)).toBe("Last 2 sessions");
  });

  it("formats the date in the interface language's order", () => {
    ui.set("en");
    expect(formatDate(at(2026, 1, 5, 7, 3))).toBe("01/05/2026 07:03");
    ui.set("de");
    expect(formatDate(at(2026, 1, 5, 7, 3))).toBe("05.01.2026 07:03");
    ui.set("es");
    expect(formatDate(at(2026, 1, 5, 7, 3))).toBe("05/01/2026 07:03");
  });
});
