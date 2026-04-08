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

  const sanitizedHtml = useMemo(() => {
    DOMPurify.addHook("afterSanitizeAttributes", (node) => {
      if (node.tagName === "A") {
        node.setAttribute("target", "_blank");
        node.setAttribute("rel", "noopener noreferrer");
      }
    });
    const html = DOMPurify.sanitize(problem.description, { ADD_ATTR: ["target"] });
    DOMPurify.removeHook("afterSanitizeAttributes");
    return html;
  }, [problem.description]);

  return (
    <div className="flex flex-col h-full w-full overflow-hidden" style={{ borderRight: "1px solid var(--border)" }}>
      {/* Tabs */}
      <div className="flex gap-1 border-b px-4 shrink-0" style={{ background: "var(--surface-2)", borderColor: "var(--border)" }}>
        {(["description", "testcases"] as const).map(t => (
          <button
            key={t}
            onClick={() => setTab(t)}
            className="text-xs px-3 py-2.5 font-medium relative"
            style={{
              color: tab === t ? "var(--text)" : "var(--muted)",
              background: "transparent",
              cursor: "pointer",
            }}
          >
            {t === "description" ? "Description" : "Test Cases"}
            {tab === t && (
              <span
                className="absolute bottom-0 left-0 right-0 h-0.5 rounded-full"
                style={{ background: "var(--orange)" }}
              />
            )}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-5">
        {tab === "description" ? (
          <div
            className="problem-html"
            dangerouslySetInnerHTML={{ __html: sanitizedHtml }}
          />
        ) : (
          <div className="flex flex-col gap-3">
            {testCases.map((tc, i) => (
              <div
                key={i}
                className="rounded-lg p-4"
                style={{ background: "rgba(255,255,255,0.03)", border: "1px solid rgba(255,255,255,0.06)" }}
              >
                <div className="text-xs font-semibold mb-2" style={{ color: "var(--text)" }}>Case {i + 1}</div>
                <div className="mb-2">
                  <div className="text-xs mb-1" style={{ color: "var(--muted)" }}>Input</div>
                  <pre
                    className="text-xs rounded-md px-3 py-2"
                    style={{
                      background: "rgba(255,255,255,0.04)",
                      color: "var(--text)",
                      fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
                      whiteSpace: "pre-wrap",
                      overflowWrap: "break-word",
                    }}
                  >{tc.input}</pre>
                </div>
                {tc.expected_output && (
                  <div>
                    <div className="text-xs mb-1" style={{ color: "var(--muted)" }}>Expected</div>
                    <pre
                      className="text-xs rounded-md px-3 py-2"
                      style={{
                        background: "rgba(255,255,255,0.04)",
                        color: "var(--green)",
                        fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
                        whiteSpace: "pre-wrap",
                        overflowWrap: "break-word",
                      }}
                    >{tc.expected_output}</pre>
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
