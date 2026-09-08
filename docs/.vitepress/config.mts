import { writeFile } from "node:fs/promises"
import { join } from "node:path"
import { defineConfig, type Plugin } from "vitepress"
import llmstxt from "vitepress-plugin-llms"

import navigation from "./navigation.json" with { type: "json" }

const repository = "https://github.com/peter-gy/refkit"
const siteUrl = new URL("https://peter-gy.github.io/refkit/")
const socialImage = new URL("og.png", siteUrl).href
const description =
  "Parse, render, inspect, format, and edit bibliography data from Python, JavaScript, and Polars."
const baseName = process.env.BASE_PATH?.trim().replace(/^\/+|\/+$/g, "")
const basePath = baseName ? `/${baseName}` : ""
const publicPath = (path: string): string => `${basePath}${path}`
const llmsDomain = basePath ? siteUrl.origin : siteUrl.href.replace(/\/$/, "")
const port = process.env.PORT ? Number(process.env.PORT) : undefined
const llmsSidebar = [
  { text: "Overview", items: [{ text: "RefKit", link: "/" }] },
  ...navigation.sidebar,
]

// SAFETY: vitepress-plugin-llms returns two Vite plugins whose standard hooks
// are loaded and executed by the pinned VitePress and Vite releases.
const llmsPlugins = llmstxt({
  domain: llmsDomain,
  excludeIndexPage: false,
  sidebar: llmsSidebar,
}) as [Plugin, Plugin]

if (port !== undefined && (!Number.isInteger(port) || port < 1 || port > 65_535)) {
  throw new Error(`PORT must be an integer from 1 through 65535, received ${process.env.PORT}`)
}

export default defineConfig({
  base: basePath ? `${basePath}/` : "/",
  lang: "en-US",
  title: "RefKit",
  description,
  lastUpdated: true,
  cleanUrls: true,
  sitemap: { hostname: siteUrl.href },
  vite: {
    plugins: llmsPlugins,
    server: {
      host: "127.0.0.1",
      ...(port === undefined ? {} : { port, strictPort: true }),
    },
  },
  head: [
    ["link", { rel: "icon", href: publicPath("/favicon.ico"), sizes: "any" }],
    [
      "link",
      {
        rel: "icon",
        type: "image/svg+xml",
        media: "(prefers-color-scheme: light)",
        href: publicPath("/brand/refkit-favicon-light.svg"),
      },
    ],
    [
      "link",
      {
        rel: "icon",
        type: "image/svg+xml",
        media: "(prefers-color-scheme: dark)",
        href: publicPath("/brand/refkit-favicon-dark.svg"),
      },
    ],
    ["meta", { property: "og:type", content: "website" }],
    ["meta", { property: "og:site_name", content: "RefKit" }],
    ["meta", { property: "og:image", content: socialImage }],
    ["meta", { property: "og:image:width", content: "2400" }],
    ["meta", { property: "og:image:height", content: "1260" }],
    [
      "meta",
      {
        property: "og:image:alt",
        content: "RefKit: Parse, cite, and tidy BibTeX from Python, fast.",
      },
    ],
    ["meta", { name: "twitter:card", content: "summary_large_image" }],
    ["meta", { name: "twitter:image", content: socialImage }],
    [
      "meta",
      {
        name: "theme-color",
        media: "(prefers-color-scheme: light)",
        content: "#fafafa",
      },
    ],
    [
      "meta",
      {
        name: "theme-color",
        media: "(prefers-color-scheme: dark)",
        content: "#0a0a0a",
      },
    ],
  ],
  transformPageData(pageData) {
    const pageTitle =
      pageData.frontmatter.layout === "home"
        ? "RefKit"
        : `${pageData.title} | RefKit`
    const pageDescription = pageData.frontmatter.description ?? description
    pageData.frontmatter.head ??= []
    pageData.frontmatter.head.push(
      ["meta", { property: "og:title", content: pageTitle }],
      ["meta", { property: "og:description", content: pageDescription }],
      ["meta", { name: "twitter:title", content: pageTitle }],
      ["meta", { name: "twitter:description", content: pageDescription }],
    )

    const path = pageData.relativePath
      .replace(/(^|\/)index\.md$/, "$1")
      .replace(/\.md$/, "")
    const canonical = new URL(path, siteUrl).href
    pageData.frontmatter.head.push(
      ["link", { rel: "canonical", href: canonical }],
      ["meta", { property: "og:url", content: canonical }],
    )
  },
  async buildEnd(siteConfig) {
    await writeFile(
      join(siteConfig.outDir, "robots.txt"),
      `User-agent: *\nAllow: /\nSitemap: ${new URL("sitemap.xml", siteUrl).href}\n`,
    )
  },
  themeConfig: {
    logo: {
      light: "/brand/refkit-lockup-horizontal-light-transparent.svg",
      dark: "/brand/refkit-lockup-horizontal-dark-transparent.svg",
      alt: "RefKit",
    },
    siteTitle: false,
    nav: navigation.nav,
    sidebar: navigation.sidebar,
    socialLinks: [{ icon: "github", link: repository }],
    search: { provider: "local" },
    outline: { level: [2, 3], label: "On this page" },
    editLink: {
      pattern: `${repository}/edit/main/docs/:path`,
      text: "Edit this page on GitHub",
    },
    docFooter: { prev: "Previous", next: "Next" },
    footer: {
      message: "Released under the Apache License 2.0.",
      copyright: "RefKit",
    },
    externalLinkIcon: true,
  },
})
