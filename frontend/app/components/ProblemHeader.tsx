import { Problem, Model } from "@/app/lib/types";

interface Props {
  problem: Problem;
  newModels: Model[];
  solved: boolean;
  onRaceAgain: () => void;
  isRacing: boolean;
}

export default function ProblemHeader({ problem, newModels, solved, onRaceAgain, isRacing }: Props) {
  return (
    <div className="px-5 py-4 border-b" style={{ borderColor: "var(--border)" }}>
      <div className="flex items-center gap-2 flex-wrap mb-2">
        <span className="text-xs" style={{ color: "var(--muted)" }}>#{problem.lc_id}</span>
        <span
          className="text-xs rounded-full px-2 py-0.5 font-semibold"
          style={
            problem.difficulty === "Easy"
              ? { color: "var(--green)", background: "rgba(0,184,163,0.1)" }
              : problem.difficulty === "Medium"
              ? { color: "var(--orange)", background: "rgba(255,161,22,0.1)" }
              : { color: "var(--red)", background: "rgba(239,71,67,0.1)" }
          }
        >
          {problem.difficulty}
        </span>
        {solved && (
          <span
            className="text-xs rounded-full px-2 py-0.5 font-semibold"
            style={{ color: "var(--green)", background: "rgba(0,184,163,0.1)" }}
          >
            Solved
          </span>
        )}
        {newModels.length > 0 && (
          <span
            className="ml-auto text-xs rounded px-2 py-0.5"
            style={{ background: "#1a1a00", color: "#ffdd57" }}
          >
            🆕 {newModels[0].display_name} just dropped
          </span>
        )}
        <button
          onClick={onRaceAgain}
          disabled={isRacing}
          className="text-xs rounded px-2 py-0.5"
          style={{
            color: isRacing ? "var(--muted)" : "var(--orange)",
            border: `1px solid ${isRacing ? "var(--border)" : "var(--orange)"}`,
            background: "transparent",
            cursor: isRacing ? "not-allowed" : "pointer",
            marginLeft: newModels.length > 0 ? "0" : "auto",
          }}
        >
          {isRacing ? "running…" : "▶ re-run benchmarks"}
        </button>
      </div>
      <div className="text-xl font-bold mb-1" style={{ color: "var(--text)" }}>
        {problem.title}
      </div>
      <div className="text-sm font-semibold" style={{ color: "var(--orange)" }}>
        How fast would AI solve this?
      </div>
    </div>
  );
}
