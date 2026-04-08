"use client";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTimer } from "@/app/hooks/useTimer";
import { formatTime } from "@/app/lib/api";
import { Problem, RaceResultWithModel } from "@/app/lib/types";

interface Props {
  problem: Problem;
  results: RaceResultWithModel[];
  onSolve: (ms: number) => void;
  onGiveUp: (ms: number) => void;
  userResult: { ms: number; gaveUp: boolean } | null;
}

type RacePhase = "idle" | "racing" | "submitted";

// Top 3 AI results (non-human, solved, sorted by time)
function getTopAIs(results: RaceResultWithModel[]): RaceResultWithModel[] {
  return results
    .filter(r => !r.is_human && r.solved && r.time_ms != null)
    .sort((a, b) => (a.time_ms ?? 0) - (b.time_ms ?? 0))
    .slice(0, 3);
}

function getRoastText(
  phase: RacePhase,
  solvedIds: Set<string>,
  topAIs: RaceResultWithModel[]
): string | null {
  if (phase === "submitted") return "Noted.";
  if (phase === "idle") return null;
  // racing
  if (solvedIds.size === 0) {
    const fastest = topAIs[0];
    if (!fastest) return null;
    return `${fastest.display_name} finishes in ${((fastest.time_ms ?? 0) / 1000).toFixed(1)}s.`;
  }
  if (solvedIds.size === topAIs.length) return "All models have submitted. No pressure.";
  const firstSolved = topAIs.find(r => solvedIds.has(r.model_id));
  return firstSolved ? `${firstSolved.display_name} has already moved on with its life.` : null;
}

export default function RaceEditor({ problem, results, onSolve, onGiveUp, userResult }: Props) {
  const [phase, setPhase] = useState<RacePhase>("idle");
  const [solvedIds, setSolvedIds] = useState<Set<string>>(new Set());
  const [code, setCode] = useState(problem.starter_code ?? "");
  const timeoutRefs = useRef<ReturnType<typeof setTimeout>[]>([]);
  const { state: timerState, elapsedMs, start, stop, reset } = useTimer();

  const topAIs = useMemo(() => getTopAIs(results), [results]);

  // Reset when problem changes
  useEffect(() => {
    setPhase("idle");
    setSolvedIds(new Set());
    setCode(problem.starter_code ?? "");
    reset();
    timeoutRefs.current.forEach(clearTimeout);
    timeoutRefs.current = [];
  }, [problem.id, reset]);

  // Clear timeouts on unmount
  useEffect(() => {
    return () => { timeoutRefs.current.forEach(clearTimeout); };
  }, []);

  const startRace = useCallback(() => {
    if (phase !== "idle") return;
    setPhase("racing");
    start();
    // Schedule each AI's "solved" event at their benchmarked time
    topAIs.forEach(ai => {
      const t = setTimeout(() => {
        setSolvedIds(prev => new Set([...prev, ai.model_id]));
      }, ai.time_ms!);
      timeoutRefs.current.push(t);
    });
  }, [phase, start, topAIs]);

  const handleInput = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setCode(e.target.value);
    startRace();
  };

  const handleSubmit = () => {
    const ms = stop();
    setPhase("submitted");
    onSolve(ms);
  };

  const handleGiveUp = () => {
    const ms = stop();
    setPhase("submitted");
    onGiveUp(ms);
  };

  const roastText = getRoastText(phase, solvedIds, topAIs);
  const maxAITime = topAIs.length > 0 ? (topAIs[topAIs.length - 1].time_ms ?? 1) : 1;
  const userPct = timerState === "running" && maxAITime > 0
    ? Math.min(100, (elapsedMs / maxAITime) * 100)
    : timerState === "stopped"
    ? Math.min(100, ((userResult?.ms ?? elapsedMs) / maxAITime) * 100)
    : 0;

  return (
    <div className="flex flex-col h-full">

      {/* Race panel */}
      <div className="px-5 py-3 border-b" style={{ background: "var(--surface-2)", borderColor: "var(--border)" }}>
        <div className="flex items-center justify-between mb-3">
          <span className="text-xs font-semibold" style={{ color: "var(--muted)", letterSpacing: "0.15em", textTransform: "uppercase" }}>
            Live Race
          </span>
          {phase !== "idle" && (
            <span className="text-xs" style={{ color: "var(--muted)" }}>
              Your time:{" "}
              <span className="font-bold" style={{ color: "var(--orange)", fontVariantNumeric: "tabular-nums" }}>
                {timerState === "stopped" && userResult
                  ? formatTime(userResult.ms)
                  : formatTime(elapsedMs)}
              </span>
            </span>
          )}
        </div>

        <div className="flex flex-col gap-2">
          {/* You row */}
          <div className="flex items-center gap-3">
            <span className="text-xs font-semibold w-20 flex-shrink-0" style={{ color: "var(--orange)" }}>You</span>
            <div className="flex-1 h-1.5 rounded overflow-hidden" style={{ background: "#3a3a3a" }}>
              <div
                className="h-full rounded"
                style={{ width: `${userPct}%`, background: "var(--orange)", transition: "width 0.1s linear" }}
              />
            </div>
            <span className="text-xs font-bold w-14 text-right flex-shrink-0" style={{ color: phase === "idle" ? "#555" : "var(--orange)", fontVariantNumeric: "tabular-nums" }}>
              {phase === "idle" ? "–" : timerState === "stopped" && userResult ? formatTime(userResult.ms) : formatTime(elapsedMs)}
            </span>
          </div>

          {/* AI rows */}
          {topAIs.map(ai => {
            const solved = solvedIds.has(ai.model_id);
            const targetPct = (ai.time_ms! / maxAITime) * 100;
            const isFinished = phase === "submitted";
            return (
              <div key={ai.model_id} className="flex items-center gap-3">
                <span className="text-xs font-semibold w-20 flex-shrink-0 truncate" style={{ color: solved || isFinished ? "var(--orange)" : "var(--text)" }}>
                  {ai.display_name}
                </span>
                <div className="flex-1 h-1.5 rounded overflow-hidden" style={{ background: "#3a3a3a" }}>
                  <div
                    className="h-full rounded"
                    style={{
                      width: phase === "idle" ? "0%" : `${targetPct}%`,
                      background: solved || isFinished ? "var(--orange)" : "#5c5c5c",
                      transition: isFinished ? "none" : phase === "idle" ? "none" : `width ${(ai.time_ms! / 1000).toFixed(2)}s linear`,
                    }}
                  />
                </div>
                <span
                  className="text-xs font-bold w-14 text-right flex-shrink-0"
                  style={{ color: solved || isFinished ? "var(--orange)" : "#555", fontVariantNumeric: "tabular-nums" }}
                >
                  {phase === "idle" ? "–" : solved || isFinished ? formatTime(ai.time_ms) : "…"}
                </span>
              </div>
            );
          })}
        </div>
      </div>

      {/* Roast banner */}
      {roastText && (
        <div
          className="px-5 py-2 text-xs italic"
          style={{
            background: "#1e1000",
            borderLeft: "3px solid var(--orange)",
            borderTop: "1px solid #3a2800",
            borderRight: "1px solid #3a2800",
            borderBottom: "1px solid #3a2800",
            color: "var(--muted)",
          }}
        >
          {roastText}
        </div>
      )}

      {/* Editor toolbar */}
      <div className="flex items-center gap-3 px-4 py-2 border-b" style={{ background: "#2d2d2d", borderColor: "var(--border)" }}>
        <select
          className="text-xs rounded px-2 py-1"
          style={{ background: "#3a3a3a", border: "1px solid #4a4a4a", color: "var(--text)" }}
        >
          <option>Python 3</option>
          <option>JavaScript</option>
          <option>TypeScript</option>
          <option>Java</option>
          <option>C++</option>
        </select>
        {phase === "idle" && (
          <span className="text-xs ml-auto" style={{ color: "#555", fontStyle: "italic" }}>
            Start typing to begin the race.
          </span>
        )}
      </div>

      {/* Code editor */}
      <textarea
        className="flex-1 px-4 py-3 text-sm 2xl:text-base outline-none resize-none min-h-0"
        style={{
          background: "#1e1e1e",
          color: "var(--text)",
          fontFamily: "'Courier New', monospace",
          lineHeight: "1.65",
          minHeight: "12rem",
        }}
        value={code}
        onChange={handleInput}
        disabled={phase === "submitted"}
        spellCheck={false}
      />

      {/* Submit bar */}
      <div className="flex items-center gap-3 px-4 py-2.5 border-t" style={{ background: "var(--surface)", borderColor: "var(--border)" }}>
        <button
          onClick={handleSubmit}
          disabled={phase !== "racing"}
          className="rounded px-5 py-1.5 text-sm font-bold"
          style={{
            background: phase === "racing" ? "var(--orange)" : "#3a3a3a",
            color: phase === "racing" ? "#000" : "var(--muted)",
            cursor: phase === "racing" ? "pointer" : "not-allowed",
          }}
        >
          Submit
        </button>
        <span className="text-xs italic flex-1" style={{ color: "#555" }}>
          {phase === "submitted" ? "We told you so." : "We're not going to run it."}
        </span>
        {phase === "racing" && (
          <button
            onClick={handleGiveUp}
            className="text-xs"
            style={{ color: "var(--muted)", background: "transparent", border: "none", cursor: "pointer" }}
          >
            give up
          </button>
        )}
      </div>

    </div>
  );
}
