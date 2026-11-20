// Shares navigation across the quickstart and reference pages.

import { source } from "@/lib/source";
import { DocsLayout } from "fumadocs-ui/layouts/docs";

export default function Layout({ children }: LayoutProps<"/docs">) {
  return (
    <DocsLayout tree={source.getPageTree()} nav={{ title: "Latch Docs", url: "/docs" }}>
      {children}
    </DocsLayout>
  );
}
