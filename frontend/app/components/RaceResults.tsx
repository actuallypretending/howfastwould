"use client";
import { useState } from "react";
import { formatTime } from "@/app/lib/api";
import { RaceResultWithModel } from "@/app/lib/types";

interface Props {
  results: RaceResultWithModel[];
  userResult: { ms: number; gaveUp: boolean } | null;
  onSelectResult: (result: RaceResultWithModel) => void;
  onRaceAgain: () => void;
  isRacing: boolean;
}

const PROVIDER_COLORS: Record<string, string> = {
  openai: "#90caf9",
  anthropic: "#a5d6a7",
  google: "#ef9a9a",
  xai: "#fff59d",
  fireworks: "#80cbc4",
  mistral: "#80cbc4",
  deepseek: "#ce93d8",
  qwen: "#ce93d8",
  moonshot: "#ce93d8",
  doubao: "#ce93d8",
  hunyuan: "#ce93d8",
  human: "#f48fb1",
};

export default function RaceResults({ results, userResult, onSelectResult, onRaceAgain, isRacing }: Props) {
  const [expanded, setExpanded] = useState(false);

  const sorted = [...results].sort((a, b) => {
    if (a.solved && b.solved) return (a.time_ms ?? 0) - (b.time_ms ?? 0);
    if (a.solved) return -1;
    if (b.solved) return 1;
    return 0;
  });

  const maxTime = Math.max(...sorted.filter(r => r.solved).map(r => r.time_ms ?? 0), 1);
  const visible = expanded ? sorted : sorted.slice(0, 5);
  const hasMore = sorted.length > 5;

  return (
    <div className="px-5 py-4">
      <div className="flex items-center justify-between mb-3">
        <span className="text-xs" style={{ color: "var(--muted)" }}>
          {isRacing ? "$ race in progress..." : `$ ${sorted.length} contestants`}
        </span>
        <button
          onClick={onRaceAgain}
          disabled={isRacing}
          className="rounded px-4 py-1.5 text-xs font-bold"
          style={{
            background: isRacing ? "var(--surface)" : "#00ff41",
            color: isRacing ? "var(--muted)" : "#000",
            border: isRacing ? "1px solid var(--border)" : "none",
          }}
        >
          {isRacing ? "racing..." : "▶ race again"}
        </button>
      </div>

      <div className="flex flex-col gap-1 text-sm">
        {visible.map((r, i) => {
          const color = PROVIDER_COLORS[r.provider] ?? "#ccc";
          const barPct = r.solved && r.time_ms ? Math.max(8, (r.time_ms / maxTime) * 100) : 100;
          const medal = i === 0 ? "🥇" : i === 1 ? "🥈" : i === 2 ? "🥉" : null;

          return (
            <button
              key={r.model_id}
              className="flex items-center gap-3 rounded px-3 py-2 text-left w-full"
              style={{ background: r.is_human ? "#0a0a1a" : "var(--surface)" }}
              onClick={() => onSelectResult(r)}
            >
              <span style={{ color: "var(--muted)", width: 24, flexShrink: 0 }}>
                {medal ?? `${i + 1}.`}
              </span>
              <span style={{ color, width: 160, flexShrink: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                {r.display_name}
              </span>
              <div className="flex-1 h-1.5 rounded overflow-hidden" style={{ background: "#1a1a1a" }}>
                {r.solved && (
                  <div
                    className="h-full rounded"
                    style={{ width: `${barPct}%`, background: color }}
                  />
                )}
              </div>
              <span style={{ color: r.solved ? color : "#ff4444", width: 72, textAlign: "right", flexShrink: 0 }}>
                {r.solved ? formatTime(r.time_ms) : "💀 failed"}
              </span>
              <span style={{ color: "var(--muted)", fontSize: 11, flexShrink: 0 }}>
                {r.attempts > 1 ? `${r.attempts} tries` : ""}
              </span>
            </button>
          );
        })}

        {userResult && (
          <div
            className="flex items-center gap-3 rounded px-3 py-2"
            style={{ background: "#0a0a1a", border: "1px solid #2a2a4a" }}
          >
            <span style={{ color: "var(--muted)", width: 24 }}>👤</span>
            <span style={{ color: "#7dd3fc", width: 160 }}>You</span>
            <div className="flex-1 h-1.5 rounded overflow-hidden" style={{ background: "#1a1a1a" }}>
              <div className="h-full rounded" style={{ width: "40%", background: "#7dd3fc" }} />
            </div>
            <span style={{ color: userResult.gaveUp ? "#ff4444" : "#7dd3fc", width: 72, textAlign: "right" }}>
              {userResult.gaveUp ? "💀 gave up" : formatTime(userResult.ms)}
            </span>
          </div>
        )}

        {hasMore && (
          <button
            className="text-xs py-1"
            style={{ color: "var(--muted)" }}
            onClick={() => setExpanded(e => !e)}
          >
            {expanded ? "show less ↑" : `show ${sorted.length - 5} more ↓`}
          </button>
        )}
      </div>
    </div>
  );
}
