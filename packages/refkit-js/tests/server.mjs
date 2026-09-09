import { createServer } from "node:http";
import { readFile, rm } from "node:fs/promises";
import { resolve, extname, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { bundleConsumer } from "./bundle.mjs";
const packageRoot = resolve(
  process.env.REFKIT_PACKAGE_ROOT ??
    fileURLToPath(new URL("../", import.meta.url)),
);
const bundleRoot = await bundleConsumer(packageRoot);
process.on("SIGTERM", async () => {
  await rm(bundleRoot, { recursive: true, force: true });
  process.exit(0);
});
const types = {
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".html": "text/html",
  ".json": "application/json",
};
const server = createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(
      new URL(request.url, "http://localhost").pathname,
    );
    if (pathname === "/") {
      response.setHeader("Content-Type", "text/html");
      response.setHeader(
        "Content-Security-Policy",
        "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self'; worker-src 'self' blob:",
      );
      response.end(
        '<!doctype html><html lang="en"><title>RefKit browser tests</title><body><main>RefKit browser tests</main></body></html>',
      );
      return;
    }
    const root = /^\/(bundle|library)\//.test(pathname)
      ? bundleRoot
      : packageRoot;
    const path = resolve(root, `.${pathname}`);
    if (!path.startsWith(root + sep)) {
      response.writeHead(403).end();
      return;
    }
    response.setHeader(
      "Content-Type",
      types[extname(path)] ?? "application/octet-stream",
    );
    response.end(await readFile(path));
  } catch {
    response.writeHead(404).end();
  }
});
server.listen(Number(process.env.PORT ?? 4175), "127.0.0.1");
