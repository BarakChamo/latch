// Introduces Latch and routes readers to the setup and reference guides.

import { Card, Cards } from "fumadocs-ui/components/card";
import { HomeLayout } from "fumadocs-ui/layouts/home";
import Image from "next/image";
import Link from "next/link";
import mark from "./icon.png";

export default function Home() {
  return (
    <HomeLayout
      nav={{
        title: (
          <>
            <Image src={mark} alt="" width={24} height={24} className="rounded" unoptimized />
            Latch
          </>
        ),
      }}
      links={[
        { text: "Docs", url: "/docs" },
        { text: "Quickstart", url: "/docs/quickstart" },
        { text: "Compatibility", url: "/docs/hooks#where-reports-appear" },
      ]}
      githubUrl="https://github.com/BarakChamo/latch"
    >
      <section className="latch-hero px-6 pt-16 pb-12 text-center sm:pt-24 sm:pb-16">
        <h1 className="mx-auto max-w-4xl text-5xl font-semibold leading-[1.04] tracking-tight text-balance sm:text-7xl">
          Give your coding agent
          <span className="block text-violet-700 dark:text-violet-300">a feedback loop.</span>
        </h1>
        <p className="mx-auto mt-6 max-w-xl text-lg leading-relaxed text-fd-muted-foreground text-pretty">
          Latch runs your project’s linters, tests, and type checks after edits or turns.
          Passing checks stay quiet. Failures go back to the agent.
        </p>
        <nav aria-label="Get started" className="mt-8 flex flex-wrap justify-center gap-3">
          <Link
            className="rounded-lg bg-violet-700 px-5 py-3 text-sm font-semibold text-white transition-colors hover:bg-violet-800 dark:bg-violet-300 dark:text-violet-950 dark:hover:bg-violet-200"
            href="/docs/quickstart"
          >
            Get started <span aria-hidden="true">→</span>
          </Link>
          <Link
            className="rounded-lg border bg-fd-background px-5 py-3 text-sm font-semibold transition-colors hover:bg-fd-accent"
            href="/docs"
          >
            Read the docs
          </Link>
        </nav>

        <figure className="mx-auto mt-12 max-w-2xl overflow-hidden rounded-2xl border border-zinc-800 bg-zinc-950 text-left text-zinc-100 shadow-xl shadow-violet-950/10 sm:mt-16">
          <figcaption className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b border-zinc-800 px-5 py-3 font-mono text-xs text-zinc-400 sm:px-7">
            <span>Example turn-end report</span>
            <span className="text-rose-300">1 check failed</span>
          </figcaption>
          <pre className="px-5 py-6 font-mono text-xs leading-7 whitespace-pre-wrap sm:px-7 sm:text-sm"><code>
            <span className="text-violet-300">$ npm run typecheck</span>
            {"\n"}
            <span className="text-zinc-300">src/cart.ts:18:5</span>
            {"\n"}
            <span className="text-rose-300">error TS2322:</span>
            {" Type 'string' is not assignable to type 'number'."}
          </code></pre>
          <p className="border-t border-zinc-800 px-5 py-3 text-sm text-zinc-400 sm:px-7">
            The agent gets the file, line, and error it needs to fix.
          </p>
        </figure>
      </section>

      <section aria-labelledby="guides" className="mx-auto w-full max-w-5xl px-6 pb-16 sm:pb-24">
        <h2 id="guides" className="mb-5 text-2xl font-semibold tracking-tight">Explore the guides</h2>
        <Cards className="md:grid-cols-3">
          <Card title="Install a plugin" href="/docs/quickstart">
            Set up your coding agent and run your first check.
          </Card>
          <Card title="Configure checks" href="/docs/configuration">
            Choose commands, filter by changed files, and control what gets reported.
          </Card>
          <Card title="Use the standalone CLI" href="/docs/cli">
            Run the same checks from your terminal or in CI.
          </Card>
        </Cards>
      </section>
    </HomeLayout>
  );
}
