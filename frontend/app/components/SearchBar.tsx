"use client";
import { useEffect, useRef, useState } from "react";
import { searchProblems } from "@/app/lib/api";
import { Problem } from "@/app/lib/types";

interface Props {
  onSelect: (problem: Problem) => void;
  onRandom: () => void;
}

export default function SearchBar({ onSelect, onRandom }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Problem[]>([]);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (query.length < 2) { setResults([]); setOpen(false); return; }
    const t = setTimeout(async () => {
      const r = await searchProblems(query);
      setResults(r);
      setOpen(r.length > 0);
    }, 300);
    return () => clearTimeout(t);
  }, [query]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  return (
    <div ref={ref} className="relative flex items-center gap-2 flex-1" style={{ maxWidth: "360px" }}>
      <input
        className="w-full rounded px-3 py-1.5 text-sm outline-none"
        style={{
          background: "var(--surface)",
          border: "1px solid var(--border)",
          color: "var(--text)",
        }}
        placeholder="Search problems… Two Sum, #42, Hard"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onFocus={() => results.length > 0 && setOpen(true)}
      />
      <button
        onClick={onRandom}
        className="rounded px-3 py-1.5 text-sm flex-shrink-0"
        style={{ background: "var(--surface)", border: "1px solid var(--border)", color: "var(--text)" }}
      >
        🎲
      </button>

      {open && (
        <div
          className="absolute left-0 top-full z-20 rounded-b w-full text-sm mt-0.5"
          style={{ background: "var(--surface)", border: "1px solid var(--border)" }}
        >
          {results.map((p) => (
            <button
              key={p.id}
              className="flex w-full items-center gap-3 px-3 py-2 text-left border-b"
              style={{ borderColor: "var(--border)" }}
              onClick={() => { onSelect(p); setOpen(false); setQuery(""); }}
            >
              <span className="text-xs flex-shrink-0" style={{ color: "var(--muted)" }}>#{p.lc_id}</span>
              <span className="flex-1 truncate" style={{ color: "var(--text)" }}>{p.title}</span>
              <span
                className="text-xs flex-shrink-0"
                style={
                  p.difficulty === "Easy"
                    ? { color: "var(--green)" }
                    : p.difficulty === "Medium"
                    ? { color: "var(--orange)" }
                    : { color: "var(--red)" }
                }
              >
                {p.difficulty}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
