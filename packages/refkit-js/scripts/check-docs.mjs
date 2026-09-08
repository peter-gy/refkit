import { execFileSync } from "node:child_process";
import {
  cp,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import ts from "typescript";
import { packageRoot } from "./source.mjs";

const root = resolve(packageRoot, "../..");
const docs = join(root, "docs");
const temporary = await mkdtemp(join(tmpdir(), "refkit-docs-"));
try {
  await mkdir(join(temporary, "node_modules"));
  await symlink(
    packageRoot,
    join(temporary, "node_modules/refkit-js"),
    process.platform === "win32" ? "junction" : "dir",
  );
  await writeFile(
    join(temporary, "package.json"),
    JSON.stringify({ type: "module" }),
  );
  const programs = [];
  for (const relative of (await readdir(docs, { recursive: true })).sort()) {
    if (
      !relative.endsWith(".md") ||
      relative
        .split(/[\\/]/)
        .some((part) => part === "node_modules" || part === ".vitepress")
    )
      continue;
    const markdown = await readFile(join(docs, relative), "utf8");
    const snippets = [];
    for (const group of markdown.matchAll(
      /^::: code-group\n([\s\S]*?)^:::[ \t]*$/gm,
    )) {
      const blocks = [
        ...group[1].matchAll(
          /^```(python|ts) \[(Python|TypeScript)\]\n([\s\S]*?)^```[ \t]*$/gm,
        ),
      ];
      if (!blocks.length) continue;
      if (
        blocks.length !== 2 ||
        blocks[0][1] !== "python" ||
        blocks[1][1] !== "ts"
      ) {
        throw new Error(
          `${relative}: each shared example needs Python and TypeScript tabs`,
        );
      }
      snippets.push(...blocks);
    }
    if (!snippets.length) continue;
    const python = snippets.filter((match) => match[1] === "python");
    const typescript = snippets.filter((match) => match[1] === "ts");
    const directory = join(temporary, relative.replace(/\.md$/, ""));
    await mkdir(directory, { recursive: true });
    await cp(
      join(root, "packages/refkit/tests/fixtures/basic.bib"),
      join(directory, "references.bib"),
    );
    const filename = join(directory, "example.ts");
    await writeFile(filename, typescript.map((match) => match[3]).join("\n"));
    programs.push({
      relative,
      filename,
      python: python.map((match) => match[3]).join("\n"),
      first: typescript[0][3],
      expected: ["get-started.md", "guides/render-output.md"].includes(
        relative.replaceAll("\\", "/"),
      )
        ? markdown.match(/^```text\n([\s\S]*?)^```/m)?.[1]?.trim()
        : undefined,
    });
  }
  if (!programs.length)
    throw new Error("No paired documentation examples found");
  const program = ts.createProgram(
    programs.map((value) => value.filename),
    {
      target: ts.ScriptTarget.ES2022,
      module: ts.ModuleKind.NodeNext,
      strict: true,
      noEmit: true,
      types: ["node"],
      typeRoots: [join(packageRoot, "node_modules/@types")],
    },
  );
  const diagnostics = ts.getPreEmitDiagnostics(program);
  if (diagnostics.length)
    throw new Error(
      ts.formatDiagnosticsWithColorAndContext(diagnostics, {
        getCanonicalFileName: (name) => name,
        getCurrentDirectory: () => root,
        getNewLine: () => "\n",
      }),
    );
  const python = execFileSync(
    "uv",
    [
      "run",
      "--locked",
      "--package",
      "refkit",
      "--group",
      "dev",
      "python",
      "-c",
      "import sys; print(sys.executable)",
    ],
    { cwd: root, encoding: "utf8" },
  ).trim();
  for (const value of programs) {
    for (const [language, command, args] of [
      ["Python", python, ["-B", "-c", value.python]],
      ["TypeScript", process.execPath, [value.filename]],
    ]) {
      try {
        execFileSync(command, args, {
          cwd: dirname(value.filename),
          encoding: "utf8",
          stdio: ["ignore", "pipe", "pipe"],
        });
      } catch (error) {
        throw new Error(
          `${value.relative} (${language}):\n${error.stderr ?? error}`,
          { cause: error },
        );
      }
    }
    if (value.expected !== undefined) {
      const first = join(dirname(value.filename), "first.ts");
      await writeFile(first, value.first);
      const output = execFileSync(process.execPath, [first], {
        cwd: dirname(first),
        encoding: "utf8",
      });
      if (output.trim() !== value.expected)
        throw new Error(
          `${value.relative}: TypeScript output differs from the documented result`,
        );
    }
    console.log(`Verified Python and TypeScript: ${value.relative}`);
  }
} finally {
  await rm(temporary, { recursive: true, force: true });
}
