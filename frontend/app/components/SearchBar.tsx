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

  const diffColor = (d: string) =>
    d === "Easy" ? "#00ff41" : d === "Medium" ? "#ffaa00" : "#ff4444";

  return (
    <div ref={ref} className="relative flex gap-2 px-5 py-3 border-b" style={{ borderColor: "var(--border)" }}>
      <div
        className="flex flex-1 items-center gap-2 rounded px-3 py-2 text-sm"
        style={{ background: "var(--surface)", border: "1px solid var(--border)" }}
      >
        <span style={{ color: "var(--muted)" }}>$</span>
        <input
          className="flex-1 bg-transparent outline-none"
          style={{ color: "var(--text)" }}
          placeholder="search problem... Two Sum, #42, Hard, dp..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onFocus={() => results.length > 0 && setOpen(true)}
        />
      </div>
      <button
        onClick={onRandom}
        className="rounded px-3 py-2 text-sm"
        style={{ background: "var(--surface)", border: "1px solid var(--border)", color: "var(--text)" }}
      >
        🎲 random
      </button>

      {open && (
        <div
          className="absolute left-5 right-0 top-full z-10 rounded-b text-sm"
          style={{ background: "var(--surface)", border: "1px solid var(--border)", borderTop: "none", marginRight: "80px" }}
        >
          {results.map((p) => (
            <button
              key={p.id}
              className="flex w-full items-center gap-3 px-3 py-2 text-left hover:brightness-150 border-b"
              style={{ borderColor: "var(--border)" }}
              onClick={() => { onSelect(p); setOpen(false); setQuery(""); }}
            >
              <span style={{ color: "var(--muted)" }}>#{p.lc_id}</span>
              <span style={{ color: "var(--text)" }}>{p.title}</span>
              <span style={{ color: diffColor(p.difficulty), marginLeft: "auto", fontSize: "11px" }}>{p.difficulty}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
