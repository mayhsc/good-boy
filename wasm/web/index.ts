import init, { Emulator, Button } from "../pkg/wasm.js"

await init()

const WIDTH = 160
const HEIGHT = 144
const FRAME_MS = (70_224 / 4_194_304) * 1000
const MAX_CATCH_UP_FRAMES = 4
const SAVE_INTERVAL_MS = 3000
const ALLOWED_EXTENSIONS = [".gb", ".gbc"]

type Palette = { name: string; colors: [string, string, string, string] }

const PALETTES: Palette[] = [
  { name: "ICE CREAM", colors: ["#fff6d3", "#f9a875", "#eb6b6f", "#7c3f58"] },
  {
    name: "CLASSIC GREEN",
    colors: ["#e0f8d0", "#88c070", "#346856", "#081820"],
  },
  { name: "GRAYSCALE", colors: ["#ffffff", "#a0a4a0", "#686c68", "#000000"] },
  { name: "NYMPH", colors: ["#a1ef8c", "#3fac95", "#446176", "#2c2137"] },
  {
    name: "VELVET CHERRY",
    colors: ["#9775a6", "#683a68", "#412752", "#2d162c"],
  },
]

const KEYMAP: Record<string, Button> = {
  ArrowUp: Button.Up,
  ArrowDown: Button.Down,
  ArrowLeft: Button.Left,
  ArrowRight: Button.Right,
  KeyZ: Button.A,
  KeyX: Button.B,
  Enter: Button.Start,
  Backspace: Button.Select,
}

const $ = <T extends HTMLElement>(id: string) =>
  document.getElementById(id) as T

const canvas = $<HTMLCanvasElement>("screen")
const ctx = canvas.getContext("2d")!
const imageData = ctx.createImageData(WIDTH, HEIGHT)
const consoleEl = $("console")
const screenMsg = $("screen-msg")
const pausedBadge = $("paused-badge")
const led = $("led")
const cartTitle = $("cart-title")
const romInput = $<HTMLInputElement>("rom-input")
const dropOverlay = $("drop-overlay")
const toastEl = $("toast")
const btnPause = $<HTMLButtonElement>("btn-pause")
const btnReset = $<HTMLButtonElement>("btn-reset")
const btnFull = $<HTMLButtonElement>("btn-full")
const palettesEl = $("palettes")

const store = {
  get(key: string): string | null {
    try {
      return localStorage.getItem(key)
    } catch {
      return null
    }
  },
  set(key: string, value: string) {
    try {
      localStorage.setItem(key, value)
    } catch {}
  },
}

function toBase64(bytes: Uint8Array): string {
  let s = ""
  for (let i = 0; i < bytes.length; i += 0x8000) {
    s += String.fromCharCode(...bytes.subarray(i, i + 0x8000))
  }
  return btoa(s)
}

function fromBase64(b64: string): Uint8Array {
  const s = atob(b64)
  const out = new Uint8Array(s.length)
  for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i)
  return out
}

let toastTimer = 0
function toast(msg: string, kind: "ok" | "err" = "err") {
  toastEl.textContent = msg
  toastEl.className = `toast ${kind}`
  toastEl.hidden = false
  clearTimeout(toastTimer)
  toastTimer = window.setTimeout(() => (toastEl.hidden = true), 3500)
}

let paletteIndex = Math.max(
  0,
  PALETTES.findIndex((p) => p.name === store.get("goodboy:palette")),
)
let paletteBytes = new Uint8Array(12)

function hexToRgb(hex: string): [number, number, number] {
  const n = parseInt(hex.slice(1), 16)
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255]
}

function setPalette(i: number) {
  paletteIndex = (i + PALETTES.length) % PALETTES.length
  const p = PALETTES[paletteIndex]
  paletteBytes = new Uint8Array(p.colors.flatMap(hexToRgb))
  p.colors.forEach((c, n) =>
    document.documentElement.style.setProperty(`--c${n}`, c),
  )
  store.set("goodboy:palette", p.name)

  for (const el of palettesEl.children) {
    el.setAttribute(
      "aria-checked",
      String(el.getAttribute("data-index") === String(paletteIndex)),
    )
  }

  if (emu) draw()
}

for (const [i, p] of PALETTES.entries()) {
  const btn = document.createElement("button")
  btn.className = "swatch"
  btn.setAttribute("role", "radio")
  btn.dataset.index = String(i)
  btn.innerHTML = `<span class="chips">${p.colors.map((c) => `<i style="background:${c}"></i>`).join("")}</span>${p.name}`
  btn.addEventListener("click", () => setPalette(i))
  palettesEl.append(btn)
}

let emu: Emulator | null = null
let rom: Uint8Array | null = null
let saveKey = ""
let lastSaved = ""
let paused = false
let lastTick = 0
let acc = 0
let lastSaveCheck = 0

function romTitle(emu: Emulator | null): string {
  return emu ? emu.name() : "UNTITLED"
}

function romSaveKey(bytes: Uint8Array): string {
  const hex = (n: number) => (bytes[n] ?? 0).toString(16).padStart(2, "0")
  return `goodboy:save:${romTitle(emu)}:${hex(0x14d)}${hex(0x14e)}${hex(0x14f)}`
}

function persistSave() {
  if (!emu || !saveKey) return
  const data = emu.save_data()
  if (!data) return
  const b64 = toBase64(data)
  if (b64 !== lastSaved) {
    store.set(saveKey, b64)
    lastSaved = b64
  }
}

function boot(bytes: Uint8Array) {
  persistSave()
  emu?.free()
  emu = null

  const key = romSaveKey(bytes)
  const stored = store.get(key)
  let save: Uint8Array | undefined
  try {
    save = stored ? fromBase64(stored) : undefined
  } catch {
    save = undefined
  }

  try {
    emu = new Emulator(bytes, save)
  } catch (e) {
    console.error(e)
    toast("COULDN'T START THAT CARTRIDGE")
    setRunningUI(false)
    return
  }

  rom = bytes
  saveKey = key
  lastSaved = stored ?? ""
  paused = false
  acc = 0
  lastTick = performance.now()
  cartTitle.textContent = romTitle(emu)
  setRunningUI(true)
  toast(
    save ? `LOADED ${romTitle(emu)} + SAVE` : `LOADED ${romTitle(emu)}`,
    "ok",
  )
}

function setRunningUI(running: boolean) {
  screenMsg.hidden = running
  btnPause.disabled = !running
  btnReset.disabled = !running
  led.classList.toggle("on", running)
  led.classList.toggle("paused", running && paused)
  pausedBadge.hidden = !(running && paused)
  btnPause.textContent = paused ? "RESUME" : "PAUSE"
  if (!running) {
    cartTitle.textContent = "— EMPTY SLOT —"
    ctx.clearRect(0, 0, WIDTH, HEIGHT)
  }
}

function togglePause() {
  if (!emu) return
  paused = !paused
  lastTick = performance.now()
  acc = 0
  if (paused) persistSave()
  setRunningUI(true)
}

function draw() {
  if (!emu) return
  imageData.data.set(emu.rgba(paletteBytes))
  ctx.putImageData(imageData, 0, 0)
}

function crash(e: unknown) {
  console.error(e)
  emu = null
  setRunningUI(false)
  toast("EMULATOR CRASHED — SEE CONSOLE")
}

function loop(now: number) {
  requestAnimationFrame(loop)
  if (!emu || paused) return

  acc += now - lastTick
  lastTick = now
  if (acc > FRAME_MS * MAX_CATCH_UP_FRAMES) acc = FRAME_MS * MAX_CATCH_UP_FRAMES

  let ran = false
  try {
    while (acc >= FRAME_MS) {
      emu.run_frame()
      acc -= FRAME_MS
      ran = true
    }
    if (ran) draw()
  } catch (e) {
    crash(e)
    return
  }

  if (now - lastSaveCheck > SAVE_INTERVAL_MS) {
    lastSaveCheck = now
    persistSave()
  }
}

requestAnimationFrame(loop)

const held = new Map<Button, Set<string>>()
const padEls = new Map<Button, HTMLElement[]>()

for (const el of document.querySelectorAll<HTMLElement>("[data-btn]")) {
  const btn = Button[el.dataset.btn as keyof typeof Button]
  padEls.set(btn, [...(padEls.get(btn) ?? []), el])

  const source = `ptr:${el.dataset.btn}`
  el.addEventListener("pointerdown", (e) => {
    e.preventDefault()
    el.setPointerCapture(e.pointerId)
    hold(btn, source, true)
  })
  for (const type of [
    "pointerup",
    "pointercancel",
    "lostpointercapture",
  ] as const) {
    el.addEventListener(type, () => hold(btn, source, false))
  }
  el.addEventListener("contextmenu", (e) => e.preventDefault())
}

function hold(btn: Button, source: string, down: boolean) {
  const set = held.get(btn) ?? new Set<string>()
  const wasDown = set.size > 0
  if (down) set.add(source)
  else set.delete(source)
  held.set(btn, set)
  const isDown = set.size > 0

  for (const el of padEls.get(btn) ?? []) el.classList.toggle("pressed", isDown)

  if (!emu || wasDown === isDown) return
  if (isDown) emu.press(btn)
  else emu.release(btn)
}

function releaseAll() {
  for (const [btn, set] of held) {
    for (const s of [...set]) hold(btn, s, false)
  }
}

window.addEventListener("keydown", (e) => {
  if (e.metaKey || e.ctrlKey || e.altKey) return
  const btn = KEYMAP[e.code]
  if (btn !== undefined) {
    e.preventDefault()
    if (!e.repeat) hold(btn, `key:${e.code}`, true)
    return
  }
  if (e.repeat) return
  if (e.code === "KeyP") togglePause()
  if (e.code === "KeyC") setPalette(paletteIndex + 1)
})

window.addEventListener("keyup", (e) => {
  const btn = KEYMAP[e.code]
  if (btn !== undefined) hold(btn, `key:${e.code}`, false)
})

window.addEventListener("blur", releaseAll)

async function loadFile(file: File) {
  const ext = file.name.slice(file.name.lastIndexOf(".")).toLowerCase()
  if (!ALLOWED_EXTENSIONS.includes(ext)) {
    toast("THAT'S NOT A .GB OR .GBC FILE")
    return
  }
  boot(new Uint8Array(await file.arrayBuffer()))
}

romInput.addEventListener("change", () => {
  const file = romInput.files?.[0]
  if (file) void loadFile(file)
  romInput.value = ""
})

$("screen-wrap").addEventListener("click", () => {
  if (!emu) romInput.click()
})

let dragDepth = 0
window.addEventListener("dragenter", (e) => {
  if (!e.dataTransfer?.types.includes("Files")) return
  e.preventDefault()
  dragDepth++
  dropOverlay.hidden = false
})
window.addEventListener("dragleave", () => {
  dragDepth = Math.max(0, dragDepth - 1)
  if (dragDepth === 0) dropOverlay.hidden = true
})
window.addEventListener("dragover", (e) => e.preventDefault())
window.addEventListener("drop", (e) => {
  e.preventDefault()
  dragDepth = 0
  dropOverlay.hidden = true
  const file = e.dataTransfer?.files[0]
  if (file) void loadFile(file)
})

btnPause.addEventListener("click", togglePause)

btnReset.addEventListener("click", () => {
  if (rom) boot(rom)
})

btnFull.addEventListener("click", () => {
  if (document.fullscreenElement) void document.exitFullscreen()
  else void consoleEl.requestFullscreen?.()
})

document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    persistSave()
    releaseAll()
  } else {
    lastTick = performance.now()
    acc = 0
  }
})
window.addEventListener("pagehide", persistSave)

setPalette(paletteIndex)
setRunningUI(false)
