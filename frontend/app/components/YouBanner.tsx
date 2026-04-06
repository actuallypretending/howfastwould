"use client";
import { useEffect } from "react";
import { useTimer } from "@/app/hooks/useTimer";
import { formatTime } from "@/app/lib/api";

interface Props {
  problemId: string;
  onSolve: (ms: number) => void;
  onGiveUp: (ms: number) => void;
}

export default function YouBanner({ problemId, onSolve, onGiveUp }: Props) {
  const { state, elapsedMs, start, stop, reset } = useTimer();

  useEffect(() => {
    reset();
    start();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [problemId]);

  const handleSolve = () => {
    const ms = stop();
    onSolve(ms);
  };

  const handleGiveUp = () => {
    const ms = stop();
    onGiveUp(ms);
  };

  const display = state === "stopped"
    ? `✓ ${formatTime(elapsedMs)}`
    : formatTime(elapsedMs);

  return (
    <div
      className="mx-5 mt-4 flex items-center gap-4 rounded px-4 py-3"
      style={{ background: "#0a0a1a", border: "1px solid #2a2a4a" }}
    >
      <div>
        <div className="text-xs mb-1" style={{ color: "var(--muted)" }}>⏱ your time</div>
        <div className="text-2xl font-black tracking-widest" style={{ color: state === "stopped" ? "#00ff41" : "#fff" }}>
          {display}
        </div>
      </div>
      <div className="flex-1 text-xs" style={{ color: "var(--muted)" }}>
        {state === "running" && "Timer started when you opened the problem."}
        {state === "stopped" && "Timer stopped. Your result is in the leaderboard."}
      </div>
      {state === "running" && (
        <>
          <button
            onClick={handleSolve}
            className="rounded px-4 py-2 text-sm font-bold"
            style={{ background: "#00ff41", color: "#000" }}
          >
            ✓ I solved it
          </button>
          <button
            onClick={handleGiveUp}
            className="rounded px-3 py-2 text-sm"
            style={{ background: "transparent", border: "1px solid #333", color: "var(--muted)" }}
          >
            give up 💀
          </button>
        </>
      )}
    </div>
  );
}
