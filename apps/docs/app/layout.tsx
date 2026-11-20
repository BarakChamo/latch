// Provides metadata and the global theme for the static docs app.

import type { Metadata } from "next";
import { Provider } from "@/components/provider";
import "./global.css";

export const metadata: Metadata = {
  title: { default: "Latch Docs", template: "%s | Latch Docs" },
  description: "Configure project checks for your coding harness.",
};

export default function Layout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>
        <Provider>{children}</Provider>
      </body>
    </html>
  );
}
