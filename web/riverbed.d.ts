/* tslint:disable */
/* eslint-disable */
export function setup(): void;
/**
 * Chroma subsampling format
 */
export enum ChromaSampling {
  /**
   * Both vertically and horizontally subsampled.
   */
  Cs420 = 0,
  /**
   * Horizontally subsampled.
   */
  Cs422 = 1,
  /**
   * Not subsampled.
   */
  Cs444 = 2,
  /**
   * Monochrome.
   */
  Cs400 = 3,
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly setup: () => void;
  readonly main: (a: number, b: number) => number;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_1: WebAssembly.Table;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_export_6: WebAssembly.Table;
  readonly closure12800_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure12797_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure12804_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure12808_externref_shim: (a: number, b: number, c: any) => void;
  readonly _dyn_core__ops__function__FnMut__A____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h8ea1d89af6d37d18: (a: number, b: number, c: number) => void;
  readonly closure35866_externref_shim: (a: number, b: number, c: any) => void;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__hf8cbced170bc5b29: (a: number, b: number) => void;
  readonly closure35854_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure35857_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure35860_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure35878_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure35881_externref_shim: (a: number, b: number, c: any, d: any) => void;
  readonly closure35875_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure35869_externref_shim: (a: number, b: number, c: any) => void;
  readonly _dyn_core__ops__function__FnMut_____Output___R_as_wasm_bindgen__closure__WasmClosure___describe__invoke__h30c64cbd9f3f241c: (a: number, b: number) => void;
  readonly closure171277_externref_shim: (a: number, b: number, c: any) => void;
  readonly closure180308_externref_shim: (a: number, b: number, c: any) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
