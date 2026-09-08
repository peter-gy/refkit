import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { createInterface } from "node:readline";
import { pathToFileURL } from "node:url";
import { dirname, join } from "node:path";

const modulePath = process.argv[2];
const { tidy } = await import(pathToFileURL(modulePath).href);
const packageInfo = JSON.parse(await readFile(join(dirname(modulePath), "package.json"), "utf8"));
const artifact = await readFile(modulePath);
const reply = (value) => process.stdout.write(`${JSON.stringify(value)}\n`);
reply({
  node_version: process.versions.node,
  v8_version: process.versions.v8,
  package_version: packageInfo.version,
  artifact_sha256: createHash("sha256").update(artifact).digest("hex"),
  clock: "process.hrtime.bigint",
  node_flags_sha256: createHash("sha256")
    .update(JSON.stringify([process.execArgv, process.env.NODE_OPTIONS ?? ""]))
    .digest("hex"),
});
let input;
let options;
for await (const line of createInterface({ input: process.stdin })) {
  try {
    const request = JSON.parse(line);
    if (request.input !== undefined) {
      input = request.input;
      options = request.options;
      reply({ ready: true });
      continue;
    }
    const loops = request.loops;
    if (!Number.isSafeInteger(loops) || loops < 1 || typeof input !== "string") {
      throw new Error("Expected a configured input and a positive safe-integer loop count");
    }
    let result;
    const started = process.hrtime.bigint();
    for (let index = 0; index < loops; index++) result = tidy(input, options);
    const elapsed = Number(process.hrtime.bigint() - started) / 1e9;
    reply({ elapsed, result });
  } catch (error) {
    reply({ error: String(error) });
  }
}
