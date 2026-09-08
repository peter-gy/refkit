import initialize, {
  build_info,
  initSync,
  type InitInput,
  type SyncInitInput,
} from "./wasm/refkit_js_native.js";
import { RefkitError, readNative } from "./errors.js";
import { version } from "./wasm/version.js";

export type InitOptions =
  | InitInput
  | Promise<InitInput>
  | { module_or_path: InitInput | Promise<InitInput> };
const supportsFinalization = typeof FinalizationRegistry === "function";
let ready = false;
let pending: Promise<void> | undefined;

/** Initialize the shared WebAssembly module before calling the browser API. */
export function init(input?: InitOptions): Promise<void> {
  assertRuntimeSupport();
  if (ready) return Promise.resolve();
  if (!pending) {
    const moduleOrPath =
      input && typeof input === "object" && "module_or_path" in input
        ? input.module_or_path
        : (input ??
          new URL("./wasm/refkit_js_native_bg.wasm", import.meta.url));
    pending = initialize({ module_or_path: moduleOrPath })
      .then(() => {
        verifyVersion();
        ready = true;
      })
      .catch((error: unknown) => {
        pending = undefined;
        throw error;
      });
  }
  return pending;
}

export function initializeSync(module: SyncInitInput): void {
  assertRuntimeSupport();
  if (ready) return;
  if (pending) {
    throw new RefkitError(
      "Await the pending browser initialization before importing refkit-js/node.",
    );
  }
  initSync({ module });
  verifyVersion();
  ready = true;
}

export function assertInitialized(): void {
  if (!ready)
    throw new RefkitError(
      "RefKit is not initialized. Await init() before calling the browser API.",
    );
}

export interface BuildInfo {
  readonly version: string;
  readonly buildMode: "debug" | "release";
  readonly target: string;
}

function verifyVersion(): void {
  const info = readNative<BuildInfo>(() => build_info());
  if (info.version !== version) {
    throw new RefkitError(
      `WebAssembly version ${info.version} is incompatible with refkit-js ${version}. Install JavaScript and WebAssembly from the same release.`,
    );
  }
}

export function getBuildInfo(): BuildInfo {
  assertInitialized();
  return readNative(() => build_info());
}

function assertRuntimeSupport(): void {
  if (!supportsFinalization) {
    throw new RefkitError(
      "RefKit requires FinalizationRegistry. Use Node.js 22.19 or newer, or update your browser.",
    );
  }
}
