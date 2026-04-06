import { Problem, Model } from "@/app/lib/types";

interface Props {
  problem: Problem;
  newModels: Model[];
}

const diffStyle = {
  Easy: { background: "#1b2a1b", color: "#00ff41" },
  Medium: { background: "#2a1f0a", color: "#ffaa00" },
  Hard: { background: "#2a0a0a", color: "#ff4444" },
};

export default function ProblemHeader({ problem, newModels }: Props) {
  return (
    <div className="px-5 py-4 border-b" style={{ borderColor: "var(--border)" }}>
      <div className="flex items-center gap-2 flex-wrap mb-2">
        <span
          className="text-xs rounded px-2 py-0.5"
          style={{ background: "var(--surface)", color: "var(--muted)" }}
        >
          #{problem.lc_id}
        </span>
        <span className="text-lg font-black text-white">{problem.title}</span>
        <span
          className="text-xs rounded px-2 py-0.5"
          style={diffStyle[problem.difficulty as keyof typeof diffStyle] ?? {}}
        >
          {problem.difficulty}
        </span>
        {newModels.length > 0 && (
          <span
            className="ml-auto text-xs rounded px-2 py-0.5"
            style={{ background: "#1a1a00", color: "#ffdd57" }}
          >
            🆕 {newModels[0].display_name} just dropped
          </span>
        )}
      </div>
      <p className="text-xs leading-relaxed line-clamp-2" style={{ color: "#666" }}>
        {problem.description.replace(/<[^>]*>/g, "").slice(0, 200)}...
      </p>
    </div>
  );
}
