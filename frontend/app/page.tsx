"use client";
import { useCallback, useEffect, useState } from "react";
import { createRace, fetchModels, fetchProblemResults, fetchRandomProblem } from "./lib/api";
import { Model, Problem, RaceResultWithModel } from "./lib/types";
import MemeCard from "./components/MemeCard";
import ProblemHeader from "./components/ProblemHeader";
import RaceResults from "./components/RaceResults";
import SearchBar from "./components/SearchBar";
import YouBanner from "./components/YouBanner";

export default function Home() {
  const [problem, setProblem] = useState<Problem | null>(null);
  const [results, setResults] = useState<RaceResultWithModel[]>([]);
  const [models, setModels] = useState<Model[]>([]);
  const [isRacing, setIsRacing] = useState(false);
  const [userResult, setUserResult] = useState<{ ms: number; gaveUp: boolean } | null>(null);
  const [memeTarget, setMemeTarget] = useState<RaceResultWithModel | null>(null);
  const [roast, setRoast] = useState("");

  const loadProblem = useCallback(async (p: Problem) => {
    setProblem(p);
    setUserResult(null);
    setMemeTarget(null);
    const r = await fetchProblemResults(p.id);
    setResults(r);
  }, []);

  const loadRandom = useCallback(async () => {
    const p = await fetchRandomProblem();
    await loadProblem(p);
  }, [loadProblem]);

  useEffect(() => {
    loadRandom();
    fetchModels().then(setModels);
  }, [loadRandom]);

  const handleRaceAgain = async () => {
    if (!problem || isRacing) return;
    setIsRacing(true);
    try {
      await createRace(problem.id);
      let attempts = 0;
      const prevCount = results.length;
      const poll = setInterval(async () => {
        const r = await fetchProblemResults(problem.id);
        setResults(r);
        attempts++;
        if (attempts > 30 || r.length > prevCount) {
          clearInterval(poll);
          setIsRacing(false);
        }
      }, 3000);
    } catch {
      setIsRacing(false);
    }
  };

  const handleSelectResult = (r: RaceResultWithModel) => {
    if (!problem) return;
    setMemeTarget(r);
    const loser = results.find(x => x.model_id !== r.model_id && x.solved) ?? results[results.length - 1];
    if (loser) {
      setRoast(`${r.display_name} left ${loser.display_name} in the dust`);
    }
  };

  const newModels = models.filter(m => m.is_new);

  return (
    <div className="max-w-2xl mx-auto min-h-screen flex flex-col">
      <nav className="flex items-center justify-between px-5 py-3 border-b" style={{ borderColor: "var(--border)" }}>
        <div className="font-black text-base">
          how<span style={{ color: "#00ff41" }}>fast</span>would
          <span style={{ color: "var(--muted)" }}>.com</span>
        </div>
        <div className="flex gap-4 text-xs" style={{ color: "var(--muted)" }}>
          <span>leaderboard</span>
          <span>history</span>
          <span>about</span>
        </div>
      </nav>

      <SearchBar onSelect={loadProblem} onRandom={loadRandom} />

      {problem && (
        <>
          <ProblemHeader problem={problem} newModels={newModels} />
          <YouBanner
            problemId={problem.id}
            onSolve={(ms) => setUserResult({ ms, gaveUp: false })}
            onGiveUp={(ms) => setUserResult({ ms, gaveUp: true })}
          />
          <RaceResults
            results={results}
            userResult={userResult}
            onSelectResult={handleSelectResult}
            onRaceAgain={handleRaceAgain}
            isRacing={isRacing}
          />
        </>
      )}

      {!problem && (
        <div className="flex-1 flex items-center justify-center text-sm" style={{ color: "var(--muted)" }}>
          loading...
        </div>
      )}

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
