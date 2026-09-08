import { execFileSync } from "node:child_process";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { resolve } from "node:path";
import { fingerprint, packageRoot } from "./source.mjs";

const root = resolve(packageRoot, "../..");
const manifest = resolve(packageRoot, "rust/Cargo.toml");
function run(command, args, options = {}) {
  execFileSync(command, args, { cwd: root, stdio: "inherit", ...options });
}
const metadata = JSON.parse(
  execFileSync(
    "rustup",
    [
      "run",
      "stable",
      "cargo",
      "metadata",
      "--manifest-path",
      manifest,
      "--locked",
      "--format-version",
      "1",
    ],
    { cwd: root, encoding: "utf8" },
  ),
);
const binding = metadata.packages.find((pkg) => pkg.name === "wasm-bindgen");
const installed = execFileSync("wasm-bindgen", ["--version"], {
  encoding: "utf8",
}).trim();
if (installed !== `wasm-bindgen ${binding.version}`) {
  throw new Error(
    `Install matching build tool: cargo install wasm-bindgen-cli --version ${binding.version} --locked`,
  );
}
const sysroot = execFileSync(
  "rustup",
  ["run", "stable", "rustc", "--print", "sysroot"],
  { encoding: "utf8" },
).trim();
const remaps = [
  `--remap-path-prefix=${root}=refkit`,
  `--remap-path-prefix=${sysroot}=rust-toolchain`,
];
if (process.env.HOME)
  remaps.push(`--remap-path-prefix=${process.env.HOME}=home`);
run(
  "rustup",
  [
    "run",
    "stable",
    "cargo",
    "build",
    "--manifest-path",
    manifest,
    "--locked",
    "-p",
    "refkit-js-native",
    "--target",
    "wasm32-unknown-unknown",
    "--release",
  ],
  {
    env: {
      ...process.env,
      RUSTC: resolve(sysroot, "bin/rustc"),
      CARGO_ENCODED_RUSTFLAGS: [
        ...(process.env.CARGO_ENCODED_RUSTFLAGS?.split("\u001f") ??
          process.env.RUSTFLAGS?.split(/\s+/).filter(Boolean) ??
          []),
        ...remaps,
      ].join("\u001f"),
    },
  },
);
const generated = resolve(packageRoot, "src/wasm");
await rm(generated, { recursive: true, force: true });
await mkdir(generated, { recursive: true });
run("wasm-bindgen", [
  resolve(
    metadata.target_directory,
    "wasm32-unknown-unknown/release/refkit_js_native.wasm",
  ),
  "--target",
  "web",
  "--out-dir",
  generated,
]);
const { version } = JSON.parse(
  await readFile(resolve(packageRoot, "package.json"), "utf8"),
);
await writeFile(
  resolve(generated, "version.js"),
  `export const version = ${JSON.stringify(version)};\n`,
);
await writeFile(
  resolve(generated, "version.d.ts"),
  `export declare const version: ${JSON.stringify(version)};\n`,
);
await rm(resolve(packageRoot, "dist"), { recursive: true, force: true });
run(process.execPath, [
  resolve(packageRoot, "node_modules/typescript/bin/tsc"),
  "-p",
  resolve(packageRoot, "tsconfig.json"),
]);
await cp(generated, resolve(packageRoot, "dist/wasm"), { recursive: true });
await cp(resolve(root, "LICENSE"), resolve(packageRoot, "LICENSE"));

await writeFile(
  resolve(packageRoot, "dist/build.json"),
  `${JSON.stringify({ source: await fingerprint(), wasmBindgen: binding.version }, null, 2)}\n`,
);
run(process.execPath, [
  fileURLToPath(new URL("./check-dist.mjs", import.meta.url)),
]);
