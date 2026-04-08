"use client";
import { useEffect, useState } from "react";
import { fetchResultDetails } from "@/app/lib/api";
import { ExecutionDetails } from "@/app/lib/types";

interface Props {
  resultId: string;
}

export default function ResultDetail({ resultId }: Props) {
  const [details, setDetails] = useState<ExecutionDetails | null>(null);
  const [error, setError] = useState(false);

  useEffect(() => {
    let cancelled = false;
    fetchResultDetails(resultId)
      .then((d) => { if (!cancelled) setDetails(d); })
      .catch(() => { if (!cancelled) setError(true); });
    return () => { cancelled = true; };
  }, [resultId]);

  if (error) {
    return (
      <div className="px-5 py-3 text-xs" style={{ color: "var(--muted)" }}>
        No execution details available for this result.
      </div>
    );
  }

  if (!details) {
    return (
      <div className="px-5 py-3 text-xs" style={{ color: "var(--muted)" }}>
        Loading...
      </div>
    );
  }

  const passCount = details.test_results.filter((r) => r.passed).length;

  return (
    <div
      className="border-t"
      style={{ background: "#1a1a1a", borderColor: "var(--border)" }}
    >
      <div className="px-5 py-3">
        <div
          className="text-xs font-semibold mb-2"
          style={{ color: "var(--muted)", letterSpacing: "0.1em", textTransform: "uppercase" }}
        >
          Generated Code
        </div>
        <pre
          className="text-xs rounded p-3 overflow-x-auto"
          style={{
            background: "#0d0d0d",
            border: "1px solid var(--border)",
            color: "var(--text)",
            fontFamily: "'Courier New', monospace",
            lineHeight: "1.6",
            maxHeight: "12rem",
            overflowY: "auto",
          }}
        >
          {details.code}
        </pre>
      </div>

      <div className="px-5 pb-3">
        <div
          className="text-xs font-semibold mb-2"
          style={{ color: "var(--muted)", letterSpacing: "0.1em", textTransform: "uppercase" }}
        >
          Test Cases — {passCount}/{details.test_results.length} passed
        </div>
        <div className="flex flex-col gap-1.5">
          {details.test_results.map((tc, i) => (
            <div
              key={i}
              className="flex items-start gap-2 text-xs"
              style={{ fontFamily: "'Courier New', monospace" }}
            >
              <span
                className="flex-shrink-0"
                style={{ color: tc.passed ? "var(--green, #00d4aa)" : "var(--red)" }}
              >
                {tc.passed ? "\u2713" : "\u2717"}
              </span>
              <span style={{ color: "var(--muted)" }}>
                {tc.input} {"\u2192"} {tc.passed ? (
                  <span style={{ color: "var(--green, #00d4aa)" }}>{tc.expected}</span>
                ) : (
                  <>
                    <span style={{ color: "var(--red)", textDecoration: "line-through" }}>
                      {tc.got || "(empty)"}
                    </span>
                    {" expected "}
                    <span style={{ color: "var(--green, #00d4aa)" }}>{tc.expected}</span>
                  </>
                )}
              </span>
            </div>
          ))}
        </div>
      </div>

      {details.stderr && (
        <div
          className="mx-5 mb-3 px-3 py-2 rounded text-xs"
          style={{
            background: "rgba(239,71,67,0.08)",
            border: "1px solid rgba(239,71,67,0.2)",
            color: "var(--red)",
            fontFamily: "'Courier New', monospace",
            whiteSpace: "pre-wrap",
          }}
        >
          {details.stderr}
        </div>
      )}
    </div>
  );
}
