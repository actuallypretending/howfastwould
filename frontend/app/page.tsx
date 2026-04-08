"use client";
import { useCallback, useEffect, useRef, useState } from "react";
import Link from "next/link";
import { createRace, fetchModels, fetchProblemResults, fetchRandomProblem } from "./lib/api";
import { Model, Problem, RaceResultWithModel } from "./lib/types";
import MemeCard from "./components/MemeCard";
import ProblemHeader from "./components/ProblemHeader";
import RaceResults from "./components/RaceResults";
import SearchBar from "./components/SearchBar";
import WinnerCard from "./components/WinnerCard";
import RaceEditor from "./components/RaceEditor";

export default function Home() {
  const [problem, setProblem] = useState<Problem | null>(null);
  const [results, setResults] = useState<RaceResultWithModel[]>([]);
  const [models, setModels] = useState<Model[]>([]);
  const [isRacing, setIsRacing] = useState(false);
  const [userResult, setUserResult] = useState<{ ms: number; gaveUp: boolean } | null>(null);
  const [raceKey, setRaceKey] = useState(0);
  const [memeTarget, setMemeTarget] = useState<RaceResultWithModel | null>(null);
  const [roast, setRoast] = useState("");
  const pollRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const pollForResults = useCallback((problemId: string, currentCount: number) => {
    if (pollRef.current) clearInterval(pollRef.current);
    let attempts = 0;
    let stableCount = 0;
    let lastCount = currentCount;
    pollRef.current = setInterval(async () => {
      const r = await fetchProblemResults(problemId);
      setResults(r);
      attempts++;
      if (r.length > lastCount) { lastCount = r.length; stableCount = 0; }
      else { stableCount++; }
      if (attempts > 40 || stableCount >= 3) {
        clearInterval(pollRef.current!);
        pollRef.current = null;
        setIsRacing(false);
        setRaceKey(k => k + 1);
      }
    }, 3000);
  }, []);

  const loadProblem = useCallback(async (p: Problem) => {
    setProblem(p);
    setUserResult(null);
    setMemeTarget(null);
    const r = await fetchProblemResults(p.id);
    setResults(r);
    // Auto-poll if results look incomplete (fewer than active non-human models)
    const activeAICount = models.filter(m => !m.is_human && m.is_active).length;
    if (r.length < activeAICount && activeAICount > 0) {
      setIsRacing(true);
      pollForResults(p.id, r.length);
    }
  }, [pollForResults, models]);

  const loadRandom = useCallback(async () => {
    const p = await fetchRandomProblem();
    await loadProblem(p);
  }, [loadProblem]);

  useEffect(() => {
    loadRandom();
    fetchModels().then(setModels);
  }, [loadRandom]);

  useEffect(() => {
    return () => { if (pollRef.current) clearInterval(pollRef.current); };
  }, []);

  const handleRaceAgain = async () => {
    if (!problem || isRacing) return;
    setIsRacing(true);
    setUserResult(null);
    try {
      await createRace(problem.id);
      pollForResults(problem.id, results.length);
    } catch {
      setIsRacing(false);
      setRaceKey(k => k + 1);
    }
  };

  const handleSelectResult = (r: RaceResultWithModel) => {
    if (!problem) return;
    setMemeTarget(r);
    const loser = results.find(x => x.model_id !== r.model_id && x.solved) ?? results[results.length - 1];
    if (loser) setRoast(`${r.display_name} left ${loser.display_name} in the dust`);
  };

  const winner = results
    .filter(r => !r.is_human && r.solved && r.time_ms != null)
    .sort((a, b) => (a.time_ms ?? 0) - (b.time_ms ?? 0))[0] ?? null;

  const newModels = models.filter(m => m.is_new);

  if (!problem) {
    return (
      <div className="flex items-center justify-center min-h-screen text-sm" style={{ color: "var(--muted)" }}>
        loading...
      </div>
    );
  }

  return (
    <div className="flex flex-col" style={{ height: "100dvh" }}>

      {/* Nav */}
      <nav
        className="flex items-center gap-4 px-5 shrink-0 border-b"
        style={{ height: "2.75rem", background: "var(--surface)", borderColor: "var(--border)" }}
      >
        <div className="font-extrabold text-sm whitespace-nowrap" style={{ color: "var(--text)" }}>
          howfast<span style={{ color: "var(--orange)" }}>would</span>.com
        </div>
        <SearchBar onSelect={loadProblem} onRandom={loadRandom} />
        <div className="hidden lg:flex gap-5 text-sm ml-auto" style={{ color: "var(--muted)" }}>
          <Link href="/leaderboard" className="nav-link">Leaderboard</Link>
          <Link href="/about" className="nav-link">About</Link>
        </div>
      </nav>

      {/* Content */}
      <div className="flex flex-col lg:flex-row flex-1 min-h-0">

        {/* Left panel — problem info + leaderboard */}
        <div
          className="w-full lg:w-[26rem] lg:flex-shrink-0 lg:border-r lg:overflow-y-auto flex flex-col"
          style={{ borderColor: "var(--border)" }}
        >
          <ProblemHeader
            problem={problem}
            newModels={newModels}
            solved={userResult !== null && !userResult.gaveUp}
            onRaceAgain={handleRaceAgain}
            isRacing={isRacing}
          />
          {winner && <WinnerCard winner={winner} />}
          <RaceResults
            results={results}
            userResult={userResult}
            onSelectResult={handleSelectResult}
          />
        </div>

        {/* Right panel — race editor */}
        <div className="flex-1 flex flex-col min-h-0">
          <RaceEditor
            key={raceKey}
            problem={problem}
            results={results}
            onSolve={(ms) => setUserResult({ ms, gaveUp: false })}
            onGiveUp={(ms) => setUserResult({ ms, gaveUp: true })}
            userResult={userResult}
          />
        </div>

      </div>

      {memeTarget && problem && (
        <MemeCard
          result={memeTarget}
          problem={problem}
          roast={roast}
          onClose={() => setMemeTarget(null)}
        />
      )}
    </div>
  );
}
