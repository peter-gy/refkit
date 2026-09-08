import { execFileSync } from "node:child_process";
import {
  cp,
  mkdtemp,
  readFile,
  readdir,
  rm,
  stat,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { packageRoot } from "./source.mjs";

const npmCli = process.env.npm_execpath;
if (!npmCli) throw new Error("Run this check with npm run test:package");
const temporary = await mkdtemp(join(tmpdir(), "refkit-js-package-"));
try {
  let archive = process.argv[2] ? resolve(process.argv[2]) : undefined;
  if (!archive) {
    const output = execFileSync(
      process.execPath,
      [npmCli, "pack", "--json", "--pack-destination", temporary],
      { cwd: packageRoot, encoding: "utf8" },
    );
    const [{ filename }] = JSON.parse(output);
    archive = join(temporary, filename);
  }
  await writeFile(
    join(temporary, "package.json"),
    JSON.stringify({ private: true, type: "module" }),
  );
  execFileSync(
    process.execPath,
    [npmCli, "install", "--ignore-scripts", "--no-audit", "--no-fund", archive],
    { cwd: temporary, stdio: "inherit" },
  );
  const installed = join(temporary, "node_modules/refkit-js");
  const metadata = JSON.parse(
    await readFile(join(installed, "package.json"), "utf8"),
  );
  const expected = JSON.parse(
    await readFile(join(packageRoot, "package.json"), "utf8"),
  );
  if (metadata.name !== "refkit-js" || metadata.version !== expected.version)
    throw new Error("Installed package identity differs from release sources");
  for (const path of [
    "README.md",
    "dist/node.js",
    "dist/index.js",
    "dist/node.d.ts",
    "dist/index.d.ts",
    "dist/wasm/refkit_js_native_bg.wasm",
  ]) {
    if (!(await stat(join(installed, path))).isFile())
      throw new Error(`Missing package file ${path}`);
  }
  for (const path of await readdir(installed)) {
    if (!["LICENSE", "README.md", "package.json", "dist"].includes(path))
      throw new Error(`Unexpected package member ${path}`);
  }
  await cp(join(packageRoot, "tests"), join(temporary, "tests"), {
    recursive: true,
  });
  execFileSync(
    process.execPath,
    [
      "--expose-gc",
      "--test",
      "tests/sdk.test.mjs",
      "tests/files.test.mjs",
      "tests/lifetime.test.mjs",
    ],
    { cwd: temporary, stdio: "inherit" },
  );
  execFileSync(
    process.execPath,
    [
      join(packageRoot, "node_modules/typescript/bin/tsc"),
      "-p",
      "tests/tsconfig.json",
    ],
    { cwd: temporary, stdio: "inherit" },
  );
  console.log("Installed tarball and TypeScript consumer passed");
} finally {
  await rm(temporary, { recursive: true, force: true });
}
