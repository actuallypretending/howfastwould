"use client";
import { useMemo, useState } from "react";
import DOMPurify from "isomorphic-dompurify";
import { Problem } from "@/app/lib/types";

interface TestCase {
  input: string;
  expected_output: string;
}

interface Props {
  problem: Problem;
}

export default function ProblemPanel({ problem }: Props) {
  const [tab, setTab] = useState<"description" | "testcases">("description");

  const testCases = useMemo<TestCase[]>(() => {
    try { return JSON.parse(problem.test_cases); } catch { return []; }
  }, [problem.test_cases]);

  const sanitizedHtml = useMemo(() => DOMPurify.sanitize(problem.description), [problem.description]);

  return (
    <div className="flex flex-col h-full w-full overflow-hidden" style={{ borderRight: "1px solid var(--border)" }}>
      {/* Tabs */}
      <div className="flex border-b px-3 shrink-0" style={{ background: "var(--surface-2)", borderColor: "var(--border)" }}>
        <button
          onClick={() => setTab("description")}
          className="text-xs px-3 py-2 font-semibold"
          style={{
            color: tab === "description" ? "var(--orange)" : "var(--muted)",
            borderBottom: tab === "description" ? "2px solid var(--orange)" : "2px solid transparent",
            background: "transparent",
            cursor: "pointer",
          }}
        >
          Description
        </button>
        <button
          onClick={() => setTab("testcases")}
          className="text-xs px-3 py-2 font-semibold"
          style={{
            color: tab === "testcases" ? "var(--orange)" : "var(--muted)",
            borderBottom: tab === "testcases" ? "2px solid var(--orange)" : "2px solid transparent",
            background: "transparent",
            cursor: "pointer",
          }}
        >
          Test Cases
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4">
        {tab === "description" ? (
          <div
            className="text-sm leading-relaxed problem-html"
            style={{ color: "var(--text)" }}
            dangerouslySetInnerHTML={{ __html: sanitizedHtml }}
          />
        ) : (
          <div className="flex flex-col gap-3">
            {testCases.map((tc, i) => (
              <div key={i} className="rounded p-3 text-xs" style={{ background: "#2a2a2a", border: "1px solid var(--border)" }}>
                <div className="font-semibold mb-1" style={{ color: "var(--muted)" }}>Case {i + 1}</div>
                <div className="mb-1">
                  <span style={{ color: "var(--muted)" }}>Input: </span>
                  <code style={{ color: "var(--text)" }}>{tc.input}</code>
                </div>
                {tc.expected_output && (
                  <div>
                    <span style={{ color: "var(--muted)" }}>Expected: </span>
                    <code style={{ color: "var(--green, #00b8a3)" }}>{tc.expected_output}</code>
                  </div>
                )}
              </div>
            ))}
            {testCases.length === 0 && (
              <div className="text-xs" style={{ color: "var(--muted)" }}>No test cases available.</div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
