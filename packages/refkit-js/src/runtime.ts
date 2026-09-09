import type { InitInput, SyncInitInput } from "./wasm/refkit_js_native.js";
import { RefkitError, readNative } from "./errors.js";
import { version } from "./wasm/version.js";

type Native = typeof import("./wasm/refkit_js_native.js");
export type InitOptions =
  | InitInput
  | Promise<InitInput>
  | { module_or_path: InitInput | Promise<InitInput> };
let native: Native | undefined;
let pending: Promise<void> | undefined;

/** Load the browser bindings and WebAssembly before calling the synchronous API. */
export function init(input?: InitOptions): Promise<void> {
  assertRuntimeSupport();
  if (native) return Promise.resolve();
  if (!pending) {
    const moduleOrPath =
      input && typeof input === "object" && "module_or_path" in input
        ? input.module_or_path
        : input;
    // Attach input rejection handling while the binding chunk is loading.
    pending = Promise.all([import("./wasm/refkit_js_native.js"), moduleOrPath])
      .then(async ([bindings, source]) => {
        await bindings.default(
          source === undefined ? undefined : { module_or_path: source },
        );
        verifyVersion(bindings);
        native = bindings;
      })
      .catch((error: unknown) => {
        pending = undefined;
        throw error;
      });
  }
  return pending;
}

export function initializeSync(bindings: Native, module: SyncInitInput): void {
  assertRuntimeSupport();
  if (native) return;
  if (pending) {
    throw new RefkitError(
      "Await the pending browser initialization before importing refkit-js/node.",
    );
  }
  bindings.initSync({ module });
  verifyVersion(bindings);
  native = bindings;
}

export function getNative(): Native {
  if (!native)
    throw new RefkitError(
      "RefKit is not initialized. Await init() before calling the browser API.",
    );
  return native;
}

export interface BuildInfo {
  readonly version: string;
  readonly buildMode: "debug" | "release";
  readonly target: string;
}

function verifyVersion(bindings: Native): void {
  const info = readNative<BuildInfo>(() => bindings.build_info());
  if (info.version !== version) {
    throw new RefkitError(
      `WebAssembly version ${info.version} is incompatible with refkit-js ${version}. Install JavaScript and WebAssembly from the same release.`,
    );
  }
}

export function getBuildInfo(): BuildInfo {
  const bindings = getNative();
  return readNative(() => bindings.build_info());
}

function assertRuntimeSupport(): void {
  if (typeof FinalizationRegistry !== "function") {
    throw new RefkitError(
      "RefKit requires FinalizationRegistry. Use Node.js 22.19 or newer, or update your browser.",
    );
  }
}
