"use client";
import { useState } from "react";
import { formatTime } from "@/app/lib/api";
import { RaceResultWithModel } from "@/app/lib/types";

interface Props {
  results: RaceResultWithModel[];
  userResult: { ms: number; gaveUp: boolean } | null;
  onSelectResult: (result: RaceResultWithModel) => void;
}

const humanRoasts: Record<string, string[]> = {
  Easy: ["Probably alt-tabbing.", "Checking Stack Overflow first.", "Still reading the problem."],
  Medium: ["Getting coffee first.", "Debating whether to brute force.", "Opened 12 browser tabs."],
  Hard: ["Considering a career change.", "Praying to the DP gods.", "Questioning life choices."],
};

function HumanRow({ results }: { results: RaceResultWithModel[] }) {
  const human = results.find(r => r.is_human && r.display_name.includes("LeetCode"));
  const timeMs = human?.time_ms ?? 900_000;
  const mins = Math.round(timeMs / 60_000);
  const label = mins >= 60 ? `~${Math.round(mins / 60)}hr` : `~${mins} min`;

  const difficulty = timeMs <= 900_000 ? "Easy" : timeMs <= 2_700_000 ? "Medium" : "Hard";
  const pool = humanRoasts[difficulty];
  const roast = pool[Math.floor(Math.random() * pool.length)];

  return (
    <div className="px-5 py-2.5" style={{ background: "rgba(239,71,67,0.04)" }}>
      <div className="flex items-center gap-2 mb-1.5">
        <span className="text-xs w-4">👤</span>
        <span className="text-sm flex-1" style={{ color: "var(--muted)" }}>avg human</span>
        <span className="text-sm font-bold" style={{ color: "var(--red)" }}>{label}</span>
      </div>
      <div className="ml-6 h-1.5 rounded overflow-hidden" style={{ background: "#2e2e2e" }}>
        <div className="h-full rounded" style={{ width: "100%", background: "var(--red)", opacity: 0.35 }} />
      </div>
      <div className="ml-6 mt-1 text-xs" style={{ color: "var(--muted)" }}>{roast}</div>
    </div>
  );
}

export default function RaceResults({ results, userResult, onSelectResult }: Props) {
  const [expanded, setExpanded] = useState(false);

  const sorted = [...results]
    .filter(r => !r.is_human)
    .sort((a, b) => {
      if (a.solved && b.solved) return (a.time_ms ?? 0) - (b.time_ms ?? 0);
      if (a.solved) return -1;
      if (b.solved) return 1;
      return 0;
    });

  const maxTime = Math.max(...sorted.filter(r => r.solved).map(r => r.time_ms ?? 0), 1);

  // Insert user result at correct rank position
  const withUser: Array<RaceResultWithModel | { isUser: true; ms: number; gaveUp: boolean }> = [...sorted];
  if (userResult) {
    const insertAt = userResult.gaveUp
      ? withUser.length
      : withUser.findIndex(r => !("isUser" in r) && r.solved && (r.time_ms ?? 0) > userResult.ms);
    const idx = insertAt === -1 ? withUser.length : insertAt;
    withUser.splice(idx, 0, { isUser: true, ...userResult });
  }

  const visible = expanded ? withUser : withUser.slice(0, 5);
  const hasMore = withUser.length > 5;

  return (
    <div>
      <div
        className="px-5 py-2 text-xs font-semibold border-b"
        style={{ color: "var(--muted)", letterSpacing: "0.15em", textTransform: "uppercase", borderColor: "var(--border)" }}
      >
        All Results
      </div>

      {(() => {
        let aiRank = 0;
        return visible.map((entry, i) => {
          const isUser = "isUser" in entry;
          if (!isUser) aiRank++;
          const displayRank = aiRank;

          if (isUser) {
            const pct = entry.gaveUp ? 100 : Math.max(6, (entry.ms / maxTime) * 100);
            return (
              <div
                key="you"
                className="px-5 py-2.5 border-b"
                style={{ borderColor: "var(--border)" }}
              >
                <div className="flex items-center gap-2 mb-1.5">
                  <span className="text-xs w-4" style={{ color: "var(--muted)" }}>{i + 1}</span>
                  <span className="text-sm font-semibold flex-1" style={{ color: "var(--orange)" }}>You</span>
                  <span className="text-sm font-bold" style={{ color: entry.gaveUp ? "var(--red)" : "var(--orange)" }}>
                    {entry.gaveUp ? "gave up" : formatTime(entry.ms)}
                  </span>
                </div>
                <div className="ml-6 h-1.5 rounded overflow-hidden" style={{ background: "#2e2e2e" }}>
                  <div className="h-full rounded" style={{ width: `${pct}%`, background: entry.gaveUp ? "var(--red)" : "var(--orange)" }} />
                </div>
              </div>
            );
          }

          const r = entry as RaceResultWithModel;
          const barPct = r.solved && r.time_ms ? Math.max(6, (r.time_ms / maxTime) * 100) : 100;
          const isWinner = displayRank === 1 && r.solved;

          return (
            <button
              key={r.model_id}
              className="w-full px-5 py-2.5 border-b text-left result-row"
              style={{ borderColor: "var(--border)", cursor: "pointer", transition: "background 0.15s ease" }}
              onClick={() => onSelectResult(r)}
            >
              <div className="flex items-center gap-2 mb-1.5">
                <span className="text-xs w-4" style={{ color: "var(--muted)" }}>
                  {displayRank === 1 ? "🥇" : displayRank === 2 ? "🥈" : displayRank === 3 ? "🥉" : displayRank}
                </span>
                <span
                  className="text-sm font-semibold flex-1 truncate"
                  style={{ color: isWinner ? "var(--orange)" : "var(--text)" }}
                >
                  {r.display_name}
                </span>
                <span
                  className="text-sm font-bold flex-shrink-0"
                  style={{ color: r.solved ? (isWinner ? "var(--orange)" : "var(--muted)") : "var(--red)" }}
                >
                  {r.solved ? formatTime(r.time_ms) : "failed"}
                </span>
              </div>
              <div className="ml-6 h-1.5 rounded overflow-hidden" style={{ background: "#2e2e2e" }}>
                {r.solved && (
                  <div
                    className="h-full rounded"
                    style={{ width: `${barPct}%`, background: isWinner ? "var(--orange)" : "#5c5c5c" }}
                  />
                )}
              </div>
            </button>
          );
        });
      })()}

      {/* Avg human row — always shown as a joke reference; real human data filtered above */}
      <HumanRow results={results} />

      {hasMore && (
        <button
          className="w-full py-2 text-xs border-t"
          style={{ color: "var(--muted)", borderColor: "var(--border)" }}
          onClick={() => setExpanded(e => !e)}
        >
          {expanded ? "show less ↑" : `show ${withUser.length - 5} more ↓`}
        </button>
      )}
    </div>
  );
}
