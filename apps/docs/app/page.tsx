// Opens the quickstart at the docs site's root.

import { redirect } from "next/navigation";

export default function Home() {
  redirect("/docs");
}
