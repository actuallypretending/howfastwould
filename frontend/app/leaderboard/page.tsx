"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import { fetchLeaderboard, formatTime } from "@/app/lib/api";
import { LeaderboardEntry } from "@/app/lib/types";

export default function LeaderboardPage() {
  const [entries, setEntries] = useState<LeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchLeaderboard()
      .then((data) => { setEntries(data); setLoading(false); })
      .catch(() => { setLoading(false); });
  }, []);

  const maxWins = Math.max(...entries.map((e) => e.win_count), 1);

  return (
    <div className="flex flex-col" style={{ height: "100dvh" }}>
      {/* Nav */}
      <nav
        className="flex items-center gap-4 px-5 shrink-0 border-b"
        style={{ height: "2.75rem", background: "var(--surface)", borderColor: "var(--border)" }}
      >
        <Link href="/" className="font-extrabold text-sm whitespace-nowrap" style={{ color: "var(--text)" }}>
          howfast<span style={{ color: "var(--orange)" }}>would</span>.com
        </Link>
        <div className="flex gap-5 text-sm ml-auto" style={{ color: "var(--muted)" }}>
          <span className="font-semibold" style={{ color: "var(--orange)" }}>Leaderboard</span>
          <Link href="/about" className="nav-link">About</Link>
        </div>
      </nav>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-4xl mx-auto px-5 py-8">
          <h1 className="text-3xl font-extrabold mb-1" style={{ color: "var(--text)" }}>
            Leaderboard
          </h1>
          <p className="text-sm mb-8" style={{ color: "var(--muted)" }}>
            Aggregate performance across all benchmarked LeetCode problems.
          </p>

          {loading ? (
            <div className="text-sm" style={{ color: "var(--muted)" }}>Loading...</div>
          ) : entries.length === 0 ? (
            <div className="text-sm" style={{ color: "var(--muted)" }}>
              No benchmark data yet. Visit the homepage and run some benchmarks first.
            </div>
          ) : (
            <div className="rounded-lg overflow-hidden border overflow-x-auto" style={{ borderColor: "var(--border)" }}>
              {/* Header */}
              <div
                className="grid gap-4 px-5 py-3 text-xs font-semibold"
                style={{
                  color: "var(--muted)",
                  background: "var(--surface)",
                  letterSpacing: "0.1em",
                  textTransform: "uppercase",
                  gridTemplateColumns: "2rem 1fr 5rem 5rem 5rem 5rem 12rem",
                  minWidth: "640px",
                }}
              >
                <span>#</span>
                <span>Model</span>
                <span className="text-right">Wins</span>
                <span className="text-right">Solved</span>
                <span className="text-right">Total</span>
                <span className="text-right">Avg</span>
                <span>Solve %</span>
              </div>

              {/* Rows */}
              {entries.map((entry, i) => {
                const rank = i + 1;
                const solveRate = entry.total > 0 ? Math.round((entry.solved / entry.total) * 100) : 0;
                const winBarPct = maxWins > 0 ? (entry.win_count / maxWins) * 100 : 0;
                const medal = rank === 1 ? "🥇" : rank === 2 ? "🥈" : rank === 3 ? "🥉" : `${rank}`;
                const isTop3 = rank <= 3;

                return (
                  <div
                    key={entry.model_id}
                    className="grid gap-4 px-5 py-3 border-t result-row items-center"
                    style={{
                      borderColor: "var(--border)",
                      gridTemplateColumns: "2rem 1fr 5rem 5rem 5rem 5rem 12rem",
                      minWidth: "640px",
                      transition: "background 0.15s ease",
                    }}
                  >
                    <span className="text-sm" style={{ color: isTop3 ? "var(--orange)" : "var(--muted)" }}>
                      {medal}
                    </span>
                    <div className="min-w-0">
                      <div
                        className="text-sm font-semibold truncate"
                        style={{ color: isTop3 ? "var(--orange)" : "var(--text)" }}
                      >
                        {entry.display_name}
                      </div>
                      <div className="text-xs" style={{ color: "var(--muted)" }}>
                        {entry.provider}
                      </div>
                    </div>
                    <span
                      className="text-sm font-bold text-right"
                      style={{ color: entry.win_count > 0 ? "var(--orange)" : "var(--muted)" }}
                    >
                      {entry.win_count}
                    </span>
                    <span className="text-sm text-right" style={{ color: "var(--green)" }}>
                      {entry.solved}
                    </span>
                    <span className="text-sm text-right" style={{ color: "var(--muted)" }}>
                      {entry.total}
                    </span>
                    <span className="text-sm text-right" style={{ color: "var(--muted)" }}>
                      {entry.avg_time_ms ? formatTime(entry.avg_time_ms) : "–"}
                    </span>
                    <div className="flex items-center gap-2">
                      <div className="flex-1 h-2 rounded overflow-hidden" style={{ background: "#2e2e2e" }}>
                        <div
                          className="h-full rounded"
                          style={{
                            width: `${winBarPct}%`,
                            background: isTop3 ? "var(--orange)" : "#5c5c5c",
                            transition: "width 0.5s ease",
                          }}
                        />
                      </div>
                      <span className="text-xs w-8 text-right" style={{ color: "var(--muted)" }}>
                        {solveRate}%
                      </span>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
