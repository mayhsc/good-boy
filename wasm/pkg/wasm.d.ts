/* tslint:disable */
/* eslint-disable */

export enum Button {
    A = 0,
    B = 1,
    Start = 2,
    Select = 3,
    Up = 4,
    Down = 5,
    Left = 6,
    Right = 7,
}

export class Emulator {
    free(): void;
    [Symbol.dispose](): void;
    framebuffer(): Uint8Array;
    name(): string;
    constructor(rom: Uint8Array | number[], save?: Uint8Array | number[] | null);
    press(button: Button): void;
    release(button: Button): void;
    rgba(palette: Uint8Array | number[]): Uint8Array;
    run_frame(): void;
    save_data(): Uint8Array | undefined;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_emulator_free: (a: number, b: number) => void;
    readonly emulator_framebuffer: (a: number) => [number, number];
    readonly emulator_name: (a: number) => [number, number];
    readonly emulator_new: (a: number, b: number, c: number, d: number) => number;
    readonly emulator_press: (a: number, b: number) => void;
    readonly emulator_release: (a: number, b: number) => void;
    readonly emulator_rgba: (a: number, b: number, c: number) => [number, number];
    readonly emulator_run_frame: (a: number) => void;
    readonly emulator_save_data: (a: number) => [number, number];
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
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
