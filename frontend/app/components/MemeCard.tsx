"use client";
import { useRef } from "react";
import { formatTime } from "@/app/lib/api";
import { RaceResultWithModel, Problem } from "@/app/lib/types";

interface Props {
  result: RaceResultWithModel;
  problem: Problem;
  roast: string;
  onClose: () => void;
}

export default function MemeCard({ result, problem, roast, onClose }: Props) {
  const cardRef = useRef<HTMLDivElement>(null);

  const handleDownload = async () => {
    if (!cardRef.current) return;
    try {
      const { default: html2canvas } = await import("html2canvas");
      const canvas = await html2canvas(cardRef.current, { backgroundColor: null });
      const link = document.createElement("a");
      link.download = `howfastwould-${problem.title.replace(/\s+/g, "-").toLowerCase()}.png`;
      link.href = canvas.toDataURL();
      link.click();
    } catch {
      alert("Right-click the card to save as image, or screenshot it!");
    }
  };

  return (
    <div
      className="fixed inset-0 flex items-center justify-center z-50"
      style={{ background: "rgba(0,0,0,0.85)" }}
      onClick={onClose}
    >
      <div className="flex flex-col items-center gap-4" onClick={e => e.stopPropagation()}>
        <div
          ref={cardRef}
          className="rounded-xl p-8 text-center"
          style={{
            background: "linear-gradient(135deg, #0d0d1a, #1a0d2e)",
            width: 400,
            fontFamily: "'Impact', 'Arial Black', sans-serif",
          }}
        >
          <div style={{ fontFamily: "monospace", fontSize: 11, color: "#555", marginBottom: 8 }}>
            howfastwould.com
          </div>
          <div style={{ fontSize: 22, fontWeight: 900, color: "#ce93d8", lineHeight: 1.1, marginBottom: 8 }}>
            {result.display_name.toUpperCase()}
          </div>
          <div style={{ fontSize: 15, color: "#888", marginBottom: 4 }}>
            SOLVED {problem.title.toUpperCase()}
          </div>
          <div style={{ fontSize: 40, fontWeight: 900, color: "#fff", margin: "12px 0" }}>
            {result.solved ? formatTime(result.time_ms).toUpperCase() : "FAILED 💀"}
          </div>
          <div style={{ fontSize: 13, color: "#888", fontFamily: "monospace", fontStyle: "italic" }}>
            {roast}
          </div>
          {result.attempts > 1 && (
            <div style={{ fontSize: 11, color: "#555", marginTop: 8 }}>
              {result.attempts} attempts
            </div>
          )}
        </div>

        <div className="flex gap-3">
          <button
            onClick={handleDownload}
            className="rounded px-5 py-2 text-sm font-bold"
            style={{ background: "#00ff41", color: "#000" }}
          >
            📥 download
          </button>
          <button
            onClick={onClose}
            className="rounded px-5 py-2 text-sm"
            style={{ background: "var(--surface)", color: "var(--muted)", border: "1px solid var(--border)" }}
          >
            close
          </button>
        </div>
      </div>
    </div>
  );
}
