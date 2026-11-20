// Renders the MDX documentation as static pages.

import { source } from "@/lib/source";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { DocsBody, DocsDescription, DocsPage, DocsTitle } from "fumadocs-ui/layouts/docs/page";
import mdxComponents from "fumadocs-ui/mdx";

export function generateStaticParams() {
  return source.generateParams();
}

async function getPage(props: PageProps<"/docs/[[...slug]]">) {
  const { slug } = await props.params;
  return source.getPage(slug) ?? notFound();
}

export async function generateMetadata(props: PageProps<"/docs/[[...slug]]">): Promise<Metadata> {
  const { data } = await getPage(props);
  return { title: data.title, description: data.description ?? "" };
}

export default async function Page(props: PageProps<"/docs/[[...slug]]">) {
  const { data } = await getPage(props);
  const Content = data.body;

  return (
    <DocsPage toc={data.toc}>
      <DocsTitle>{data.title}</DocsTitle>
      <DocsDescription>{data.description}</DocsDescription>
      <DocsBody>
        <Content components={mdxComponents} />
      </DocsBody>
    </DocsPage>
  );
}
