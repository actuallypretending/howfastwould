"use client";
import { TestCaseResult } from "@/app/lib/types";

interface Props {
  results: TestCaseResult[];
  stderr: string;
}

export default function TestResultsPanel({ results, stderr }: Props) {
  if (results.length === 0) return null;

  const passCount = results.filter((r) => r.passed).length;
  const failCount = results.length - passCount;

  return (
    <div
      className="border-t"
      style={{ background: "var(--surface-2)", borderColor: "var(--border)" }}
    >
      <div
        className="flex items-center justify-between px-4 py-2 border-b"
        style={{ borderColor: "var(--border)" }}
      >
        <span className="text-xs font-semibold" style={{ color: "var(--text)" }}>
          Test Results
        </span>
        <span className="text-xs">
          <span style={{ color: "var(--green, #00d4aa)" }}>{passCount} passed</span>
          {failCount > 0 && (
            <>
              {" \u00b7 "}
              <span style={{ color: "var(--red)" }}>{failCount} failed</span>
            </>
          )}
        </span>
      </div>

      <div className="px-4 py-2 flex flex-col gap-2 max-h-48 overflow-y-auto">
        {results.map((tc, i) => (
          <div
            key={i}
            className="flex items-start gap-2 text-xs"
            style={{ fontFamily: "'Courier New', monospace" }}
          >
            <span
              className="flex-shrink-0 mt-0.5"
              style={{ color: tc.passed ? "var(--green, #00d4aa)" : "var(--red)" }}
            >
              {tc.passed ? "\u2713" : "\u2717"}
            </span>
            <div className="flex-1 min-w-0">
              <div style={{ color: "var(--muted)" }}>
                <span>Input: </span>
                <span style={{ color: "var(--text)" }}>{tc.input}</span>
              </div>
              <div style={{ color: "var(--muted)" }}>
                <span>Expected: </span>
                <span style={{ color: "var(--green, #00d4aa)" }}>{tc.expected}</span>
              </div>
              {!tc.passed && (
                <div style={{ color: "var(--muted)" }}>
                  <span>Got: </span>
                  <span
                    style={{
                      color: "var(--red)",
                      textDecoration: "line-through",
                    }}
                  >
                    {tc.got || "(empty)"}
                  </span>
                </div>
              )}
            </div>
          </div>
        ))}
      </div>

      {stderr && (
        <div
          className="mx-4 mb-2 px-3 py-2 rounded text-xs"
          style={{
            background: "rgba(239,71,67,0.08)",
            border: "1px solid rgba(239,71,67,0.2)",
            color: "var(--red)",
            fontFamily: "'Courier New', monospace",
            whiteSpace: "pre-wrap",
            maxHeight: "6rem",
            overflowY: "auto",
          }}
        >
          {stderr}
        </div>
      )}
    </div>
  );
}
