// Shares navigation across the quickstart and reference pages.

import { source } from "@/lib/source";
import { DocsLayout } from "fumadocs-ui/layouts/docs";
import Image from "next/image";
import mark from "../icon.png";

export default function Layout({ children }: LayoutProps<"/docs">) {
  return (
    <DocsLayout
      tree={source.getPageTree()}
      nav={{
        title: (
          <>
            <Image src={mark} alt="" width={24} height={24} className="rounded" unoptimized />
            Latch Docs
          </>
        ),
        url: "/docs",
      }}
    >
      {children}
    </DocsLayout>
  );
}
