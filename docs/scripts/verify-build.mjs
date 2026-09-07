import { existsSync, readFileSync, readdirSync, statSync } from "node:fs"
import { dirname, extname, join, relative, resolve } from "node:path"
import { fileURLToPath } from "node:url"

const docsRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const outputRoot = join(docsRoot, ".vitepress", "dist")
const siteUrl = new URL("https://peter-gy.github.io/refkit/")
const baseName = process.env.BASE_PATH?.trim().replace(/^\/+|\/+$/g, "")
const basePath = baseName ? `/${baseName}` : ""
const withBasePath = (path) => `${basePath}${path}`
const canonicalForRoute = (route) =>
  route === "/" ? siteUrl.href : new URL(route.replace(/^\//, ""), siteUrl).href
const markdownForRoute = (route) =>
  route === "/" ? "index.md" : `${route.replace(/^\//, "").replace(/\/$/, "/index")}.md`
const markdownUrlForRoute = (route) => new URL(markdownForRoute(route), siteUrl).href

const navigation = JSON.parse(
  readFileSync(join(docsRoot, ".vitepress", "navigation.json"), "utf8"),
)
const sidebarLinks = navigation.sidebar.flatMap((section) =>
  section.items.map((item) => item.link),
)
const configuredLinks = ["/", ...sidebarLinks, ...navigation.unlisted]
const routes = configuredLinks.map((link) =>
  link === "/" ? "index.html" : `${link.slice(1)}.html`,
)

const publicRoot = join(docsRoot, "public")
const publicFiles = readdirSync(publicRoot, { recursive: true })
  .filter((path) => statSync(join(publicRoot, path)).isFile())

const errors = []

for (const item of navigation.nav) {
  if (!sidebarLinks.includes(item.link)) {
    errors.push(`top navigation link is absent from the sidebar: ${item.link}`)
  }
}

const markdownRoutes = []
function collectMarkdown(directory) {
  for (const name of readdirSync(directory)) {
    if ([".vitepress", "node_modules", "public", "scripts"].includes(name)) continue
    const path = join(directory, name)
    if (statSync(path).isDirectory()) collectMarkdown(path)
    else if (extname(path) === ".md") {
      const source = relative(docsRoot, path)
      markdownRoutes.push(
        source === "index.md"
          ? "/"
          : `/${source.replace(/\/index\.md$/, "/").replace(/\.md$/, "")}`,
      )
    }
  }
}
collectMarkdown(docsRoot)

for (const route of markdownRoutes) {
  if (!configuredLinks.includes(route)) errors.push(`public page is absent from navigation: ${route}`)
}
for (const route of configuredLinks) {
  if (!markdownRoutes.includes(route)) errors.push(`navigation points to no public page: ${route}`)
}

for (const path of [...routes, ...publicFiles]) {
  if (!existsSync(join(outputRoot, path))) errors.push(`missing build output: ${path}`)
}

const pageDescriptions = new Set()
for (const [routeIndex, route] of routes.entries()) {
  const path = join(outputRoot, route)
  if (!existsSync(path)) continue
  const html = readFileSync(path, "utf8")
  for (const marker of [
    'property="og:title"',
    'property="og:description"',
    'name="twitter:title"',
    'name="twitter:description"',
  ]) {
    if (!html.includes(marker)) errors.push(`${route} is missing ${marker}`)
  }
  const description = html.match(/property="og:description" content="([^"]+)"/)?.[1]
  if (description) pageDescriptions.add(description)

  const routeLink = configuredLinks[routeIndex]
  const canonical = canonicalForRoute(routeLink)
  if (!html.includes(`rel="canonical" href="${canonical}"`)) {
    errors.push(`${route} has no canonical URL for ${canonical}`)
  }
  if (!html.includes(`property="og:url" content="${canonical}"`)) {
    errors.push(`${route} has no og:url for ${canonical}`)
  }
}

if (pageDescriptions.size !== routes.length) {
  errors.push(`expected one page-specific description for each of ${routes.length} routes`)
}

const sitemapPath = join(outputRoot, "sitemap.xml")
if (!existsSync(sitemapPath)) {
  errors.push("missing sitemap.xml")
} else {
  const sitemap = readFileSync(sitemapPath, "utf8")
  for (const route of configuredLinks) {
    const canonical = canonicalForRoute(route)
    if (!sitemap.includes(`<loc>${canonical}</loc>`)) {
      errors.push(`sitemap is missing ${canonical}`)
    }
  }
}

const robots = readFileSync(join(outputRoot, "robots.txt"), "utf8")
const sitemapUrl = new URL("sitemap.xml", siteUrl).href
if (!robots.includes(`Sitemap: ${sitemapUrl}`)) {
  errors.push(`robots.txt is missing ${sitemapUrl}`)
}

const markdownTwins = configuredLinks.map(markdownForRoute)
for (const path of ["llms.txt", "llms-full.txt", ...markdownTwins]) {
  const output = join(outputRoot, path)
  if (!existsSync(output) || statSync(output).size === 0) {
    errors.push(`missing or empty agent-readable output: ${path}`)
  }
}

const llms = existsSync(join(outputRoot, "llms.txt"))
  ? readFileSync(join(outputRoot, "llms.txt"), "utf8")
  : ""
const llmsFull = existsSync(join(outputRoot, "llms-full.txt"))
  ? readFileSync(join(outputRoot, "llms-full.txt"), "utf8")
  : ""
const llmsLinks = [...llms.matchAll(/\]\((https?:\/\/[^)]+\.md)\)/g)].map(
  (match) => match[1],
)
const llmsFullLinks = [
  ...llmsFull.matchAll(
    /^url:\s+(?:'(https?:\/\/[^']+\.md)'|(https?:\/\/\S+\.md)|>-\n\s+(https?:\/\/\S+\.md))$/gm,
  ),
].map((match) => match[1] ?? match[2] ?? match[3])
const expectedMarkdownLinks = configuredLinks.map(markdownUrlForRoute)

function compareExactLinks(label, actual) {
  const expected = new Set(expectedMarkdownLinks)
  const received = new Set(actual)
  for (const link of expected) {
    if (!received.has(link)) errors.push(`${label} is missing ${link}`)
  }
  for (const link of received) {
    if (!expected.has(link)) errors.push(`${label} contains unexpected ${link}`)
  }
  for (const link of received) {
    if (actual.filter((candidate) => candidate === link).length > 1) {
      errors.push(`${label} contains duplicate ${link}`)
    }
  }
}

compareExactLinks("llms.txt", llmsLinks)
compareExactLinks("llms-full.txt", llmsFullLinks)

const index = existsSync(join(outputRoot, "index.html"))
  ? readFileSync(join(outputRoot, "index.html"), "utf8")
  : ""

for (const value of [
  'property="og:image"',
  'property="og:image:width" content="2400"',
  'property="og:image:height" content="1260"',
  'name="twitter:card" content="summary_large_image"',
  withBasePath("/brand/refkit-lockup-horizontal-light-transparent.svg"),
  withBasePath("/brand/refkit-lockup-horizontal-dark-transparent.svg"),
  withBasePath("/brand/refkit-lockup-vertical-light.svg"),
  withBasePath("/brand/refkit-lockup-vertical-dark.svg"),
  withBasePath("/brand/refkit-favicon-light.svg"),
  withBasePath("/brand/refkit-favicon-dark.svg"),
  withBasePath("/icons/scan-text-light.svg"),
  withBasePath("/icons/scan-text-dark.svg"),
  withBasePath("/icons/quote-light.svg"),
  withBasePath("/icons/quote-dark.svg"),
  withBasePath("/icons/file-pen-line-light.svg"),
  withBasePath("/icons/file-pen-line-dark.svg"),
  withBasePath("/icons/table-properties-light.svg"),
  withBasePath("/icons/table-properties-dark.svg"),
  `href="${withBasePath("/performance")}" aria-label="fast, view performance evidence">fast.</a>`,
]) {
  if (!index.includes(value)) errors.push(`home metadata or asset reference is missing: ${value}`)
}

for (const path of publicFiles) {
  if (path === "robots.txt" || !existsSync(join(outputRoot, path))) continue
  if (!readFileSync(join(publicRoot, path)).equals(readFileSync(join(outputRoot, path)))) {
    errors.push(`published asset differs from source: ${path}`)
  }
}

const benchmark = JSON.parse(
  readFileSync(
    join(outputRoot, "benchmarks", "refkit-0.0.4rc5-macos-arm64-2026-09-06.json"),
    "utf8",
  ),
)
if (benchmark.rows.length !== 60) errors.push("performance evidence must contain 60 raw rows")
if (
  benchmark.rows.some(
    (row) =>
      row.status !== "ok" ||
      row.build_mode !== "release" ||
      row.source_path !== "synthetic/tiny.bib",
  )
) {
  errors.push("performance evidence has an unexpected status, build mode, or source path")
}

const performance = readFileSync(join(docsRoot, "performance.md"), "utf8")
const tableRows = [...performance.matchAll(
  /^\| `([^`]+)` \| ([^|]+?) \| ([\d,.]+) µs \|$/gm,
)]
const groupedRows = Map.groupBy(benchmark.rows, (row) => `${row.lane}|${row.package}`)
if (tableRows.length !== groupedRows.size) {
  errors.push("performance table must describe every measured lane and package once")
}
const described = new Set()
for (const [, lane, packageLabel, publishedMedian] of tableRows) {
  const matching = [...groupedRows.entries()].find(([, rows]) =>
    rows[0].lane === lane && `${rows[0].package} ${rows[0].package_version}` === packageLabel,
  )
  if (!matching || described.has(matching[0])) {
    errors.push(`performance table has an unknown or repeated measurement: ${lane} ${packageLabel}`)
    continue
  }
  const [key, rows] = matching
  described.add(key)
  const seconds = rows.map((row) => row.seconds).sort((a, b) => a - b)
  if (seconds.length !== rows[0].rounds || seconds.some((value) => !Number.isFinite(value) || value < 0)) {
    errors.push(`performance evidence has invalid measurements for ${key}`)
    continue
  }
  const middle = Math.floor(seconds.length / 2)
  const median = (seconds.length % 2 ? seconds[middle] : (seconds[middle - 1] + seconds[middle]) / 2) * 1_000_000
  if (Number(publishedMedian.replaceAll(",", "")) !== Math.round(median * 10) / 10) {
    errors.push(`performance table median does not match evidence for ${key}`)
  }
}

if (
  benchmark.metadata.refkit_commit !== "7110d53" ||
  benchmark.metadata.build_mode !== "release" ||
  benchmark.metadata.python !== "3.12.0" ||
  benchmark.metadata.packages.refkit !== "0.0.4rc5" ||
  benchmark.metadata.packages.bibtexparser !== "2.0.0b9" ||
  benchmark.metadata.packages.pybtex !== "0.26.1" ||
  benchmark.metadata.packages["citeproc-py"] !== "0.10.1" ||
  benchmark.rows.some((row) => row.rounds !== 12 || row.warmups !== 3) ||
  benchmark.rows.some(
    (row) =>
      row.setup_included !== (row.lane === "input.bibtex-text"),
  )
) {
  errors.push("performance evidence metadata or setup boundary drifted")
}

const htmlFiles = []
function collectHtml(directory) {
  for (const name of readdirSync(directory)) {
    const path = join(directory, name)
    if (statSync(path).isDirectory()) collectHtml(path)
    else if (extname(path) === ".html") htmlFiles.push(path)
  }
}

if (existsSync(outputRoot)) collectHtml(outputRoot)

const idsByFile = new Map()
for (const path of htmlFiles) {
  const html = readFileSync(path, "utf8")
  idsByFile.set(path, new Set([...html.matchAll(/\sid="([^"]+)"/g)].map((match) => match[1])))
  if (!html.includes('<html lang="en-US"')) {
    errors.push(`missing language metadata: ${relative(outputRoot, path)}`)
  }
  if (basePath) {
    for (const match of html.matchAll(/\b(?:href|src)="(\/(?!\/)[^"#?]*)/g)) {
      const reference = match[1]
      if (reference !== basePath && !reference.startsWith(`${basePath}/`)) {
        errors.push(
          `${relative(outputRoot, path)} contains root reference outside ${basePath}: ${reference}`,
        )
      }
    }
  }
}

function routeFile(from, targetPath) {
  let path = targetPath.split(/[?#]/, 1)[0]
  if (path === "" || path === "/") return join(outputRoot, "index.html")
  if (path.startsWith("/") && basePath) {
    if (path === basePath || path === `${basePath}/`) path = "/"
    else if (path.startsWith(`${basePath}/`)) path = path.slice(basePath.length)
    else return null
  }
  if (path === "/") return join(outputRoot, "index.html")
  const base = path.startsWith("/") ? outputRoot : dirname(from)
  const resolved = resolve(base, path.replace(/^\//, ""))
  if (extname(resolved)) return resolved
  return `${resolved}.html`
}

for (const source of htmlFiles) {
  const html = readFileSync(source, "utf8")
  for (const match of html.matchAll(/\shref="([^"]+)"/g)) {
    const href = match[1]
    if (/^(?:https?:|mailto:|#\/)/.test(href)) continue
    const [pathPart, fragment] = href.split("#", 2)
    const target = pathPart ? routeFile(source, pathPart) : source
    if (target === null) {
      errors.push(`${relative(outputRoot, source)} links outside deployment base: ${href}`)
      continue
    }
    if (!existsSync(target)) {
      errors.push(`${relative(outputRoot, source)} links to missing ${href}`)
      continue
    }
    if (fragment && target.endsWith(".html")) {
      const ids = idsByFile.get(target) ?? new Set()
      if (!ids.has(decodeURIComponent(fragment))) {
        errors.push(`${relative(outputRoot, source)} links to missing fragment ${href}`)
      }
    }
  }
}

if (errors.length) {
  process.stderr.write(`Documentation build verification failed:\n${errors.map((error) => `- ${error}`).join("\n")}\n`)
  process.exit(1)
}

process.stdout.write(
  `Verified ${routes.length} routes and Markdown twins, ${publicFiles.length} public files, llms indexes, ${basePath || "/"} deployment paths, metadata, theme assets, and local links.\n`,
)
