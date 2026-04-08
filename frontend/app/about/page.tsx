import Link from "next/link";

export default function AboutPage() {
  return (
    <div className="flex flex-col" style={{ height: "100dvh" }}>
      {/* Nav */}
      <nav
        className="flex items-center gap-4 px-5 shrink-0 border-b"
        style={{ height: "44px", background: "var(--surface)", borderColor: "var(--border)" }}
      >
        <Link href="/" className="font-extrabold text-sm whitespace-nowrap" style={{ color: "var(--text)" }}>
          howfast<span style={{ color: "var(--orange)" }}>would</span>.com
        </Link>
        <div className="flex gap-5 text-sm ml-auto" style={{ color: "var(--muted)" }}>
          <Link href="/leaderboard" className="nav-link">Leaderboard</Link>
          <span className="font-semibold" style={{ color: "var(--orange)" }}>About</span>
        </div>
      </nav>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-2xl mx-auto px-5 py-12">
          <h1 className="text-4xl font-extrabold mb-2" style={{ color: "var(--text)" }}>
            howfast<span style={{ color: "var(--orange)" }}>would</span>
          </h1>
          <p className="text-lg mb-8" style={{ color: "var(--muted)" }}>
            How fast would AI solve this LeetCode problem?
          </p>

          <div className="flex flex-col gap-8 text-sm leading-relaxed" style={{ color: "var(--text)" }}>
            <section>
              <h2 className="text-lg font-bold mb-3" style={{ color: "var(--orange)" }}>What is this?</h2>
              <p style={{ color: "var(--muted)" }}>
                A meme site that races AI models against each other (and you) on real LeetCode problems.
                Each model gets the problem description, generates a Python solution, and we run it against
                test cases in a sandbox. The clock is ticking.
              </p>
            </section>

            <section>
              <h2 className="text-lg font-bold mb-3" style={{ color: "var(--orange)" }}>How it works</h2>
              <ol className="flex flex-col gap-2" style={{ color: "var(--muted)" }}>
                <li className="flex gap-3">
                  <span className="font-bold flex-shrink-0" style={{ color: "var(--orange)" }}>1.</span>
                  A random LeetCode problem is fetched and shown to you.
                </li>
                <li className="flex gap-3">
                  <span className="font-bold flex-shrink-0" style={{ color: "var(--orange)" }}>2.</span>
                  AI models have already been benchmarked on it (or get benchmarked on demand).
                </li>
                <li className="flex gap-3">
                  <span className="font-bold flex-shrink-0" style={{ color: "var(--orange)" }}>3.</span>
                  Start typing to race against the AI. Your timer starts on first keypress.
                </li>
                <li className="flex gap-3">
                  <span className="font-bold flex-shrink-0" style={{ color: "var(--orange)" }}>4.</span>
                  Submit when you think you have it. We won&apos;t run your code. We trust you.
                </li>
              </ol>
            </section>

            <section>
              <h2 className="text-lg font-bold mb-3" style={{ color: "var(--orange)" }}>Models</h2>
              <p style={{ color: "var(--muted)" }}>
                We benchmark 20+ models across OpenAI, Anthropic, Google, xAI, Meta, Mistral,
                DeepSeek, Qwen, and more. Both paid frontier models and free-tier providers (Groq,
                GitHub Models, Cloudflare Workers AI). Each model gets up to 3 attempts per problem.
              </p>
            </section>

            <section>
              <h2 className="text-lg font-bold mb-3" style={{ color: "var(--orange)" }}>Fair warning</h2>
              <p style={{ color: "var(--muted)" }}>
                Benchmark times include API latency, not just thinking time. Cheaper models on faster
                infrastructure can beat smarter models that are further away. This is by design &mdash;
                in the real world, speed is speed.
              </p>
            </section>

            <div
              className="rounded-lg px-5 py-4 mt-4"
              style={{ background: "var(--surface-2)", borderLeft: "4px solid var(--orange)" }}
            >
              <p className="text-xs" style={{ color: "var(--muted)" }}>
                Built as a shitpost. Powered by Rust, Next.js, and too many API keys.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
