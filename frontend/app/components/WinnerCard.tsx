import { RaceResultWithModel } from "@/app/lib/types";
import { formatTime } from "@/app/lib/api";

interface Props {
  winner: RaceResultWithModel;
}

export default function WinnerCard({ winner }: Props) {
  return (
    <div
      className="flex items-center gap-4 px-5 py-4 border-b"
      style={{
        borderColor: "var(--border)",
        borderLeft: "4px solid var(--orange)",
        background: "var(--surface-2)",
      }}
    >
      <span className="text-2xl">🥇</span>
      <div className="flex-1 min-w-0">
        <div className="text-xs mb-1" style={{ color: "var(--muted)", letterSpacing: "0.1em", textTransform: "uppercase" }}>
          Winner
        </div>
        <div className="text-xl font-bold truncate" style={{ color: "var(--text)" }}>
          {winner.display_name}
        </div>
        <div className="text-xs mt-0.5" style={{ color: "var(--muted)" }}>
          {winner.provider}
        </div>
      </div>
      <div className="text-right flex-shrink-0">
        <div
          className="font-extrabold leading-none"
          style={{ fontSize: "3rem", color: "var(--orange)", letterSpacing: "-0.05em" }}
        >
          {formatTime(winner.time_ms)}
        </div>
        <div className="text-xs mt-1" style={{ color: "var(--muted)" }}>runtime</div>
      </div>
    </div>
  );
}
