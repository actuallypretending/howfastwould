"use client";
import { useEffect, useRef, useState } from "react";

export type TimerState = "idle" | "running" | "stopped";

export function useTimer() {
  const [state, setState] = useState<TimerState>("idle");
  const [elapsedMs, setElapsedMs] = useState(0);
  const startRef = useRef<number | null>(null);
  const rafRef = useRef<number | null>(null);

  const start = () => {
    startRef.current = Date.now();
    setState("running");
  };

  const stop = (): number => {
    const ms = startRef.current ? Date.now() - startRef.current : 0;
    setState("stopped");
    setElapsedMs(ms);
    if (rafRef.current) cancelAnimationFrame(rafRef.current);
    return ms;
  };

  const reset = () => {
    setState("idle");
    setElapsedMs(0);
    startRef.current = null;
  };

  useEffect(() => {
    if (state !== "running") return;
    const tick = () => {
      setElapsedMs(startRef.current ? Date.now() - startRef.current : 0);
      rafRef.current = requestAnimationFrame(tick);
    };
    rafRef.current = requestAnimationFrame(tick);
    return () => { if (rafRef.current) cancelAnimationFrame(rafRef.current); };
  }, [state]);

  return { state, elapsedMs, start, stop, reset };
}
