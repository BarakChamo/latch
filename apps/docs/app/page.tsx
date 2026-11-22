// Provides a readable entry point for static hosts and browsers without JavaScript.

import Image from "next/image";
import Link from "next/link";
import logo from "./latch-logo.png";

export default function Home() {
  return (
    <main className="mx-auto max-w-prose px-6 py-16">
      <h1>
        <Image
          src={logo}
          alt="Latch Docs"
          className="h-auto w-64 max-w-full rounded-lg"
          unoptimized
          preload
        />
      </h1>
      <p className="my-5">Run your project’s checks as your coding agent works and bring failures back into the session.</p>
      <ul className="list-disc space-y-2 pl-6">
        <li><Link className="underline" href="/docs/quickstart">Install a plugin and run your first check</Link></li>
        <li><Link className="underline" href="/docs/cli#install-the-cli">Install the standalone CLI</Link></li>
        <li><Link className="underline" href="/docs">Read the documentation</Link></li>
      </ul>
    </main>
  );
}
