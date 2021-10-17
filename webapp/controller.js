import initRFunge, {BefungeInterpreter} from './rfunge_wasm/rfunge.js'

let wasmInitialized = false

let ticksPerCall = 1000

export class RFungeController {
  constructor (host) {
    this._host = host
    this._mustStop = false
  }

  async init () {
    if (!wasmInitialized) {
      await initRFunge()
      wasmInitialized = true
    }
    this._interpreter = new BefungeInterpreter(this)
  }

  writeOutput (s) {
    this._host.stdout.value.write(s)
  }

  warn (msg) {}

  run () {
    new Promise((resolve, reject) => {
      const continueRunning = () => {
        if (this._mustStop) {
          this._mustStop = false
          reject(new InterpreterStopped())
          return
        }
        // Execute tickPerCall instructions and meausure how long it took
        const t0 = performance.now()
        const result = this._interpreter.run_limited(ticksPerCall)
        const t1 = performance.now()
        const dt = t1 - t0

        if (result != null) {
          // We're done!
          resolve(result)
          return
        } else {
          // Continue running
          // target 100ms per step
          const timeFactor = dt / 100
          if (timeFactor > 2) {
            // too slow
            ticksPerCall = Math.floor(ticksPerCall / timeFactor)
            console.log(`adjusting to ${ticksPerCall} ticks per iteration`)
          } else if (timeFactor < 0.5) {
            // too fast
            ticksPerCall = Math.floor(ticksPerCall / timeFactor)
            console.log(`adjusting to ${ticksPerCall} ticks per iteration`)
          }
          // go again
          setTimeout(continueRunning, 0)
        }
      }
      continueRunning()
    })
  }

  reset() {
      this._interpreter.close()
      this._interpreter = new BefungeInterpreter(this)
  }

  setSrc(src) {
      this._interpreter.loadSrc(src)
  }

  getSrc() {
      return this._interpreter.getSrc()
  }

  getSrcLines() {
      return this._interpreter.getSrcLines()
  }

  get envVars () {
    return {
      USER_AGENT: navigator.userAgent,
      HREF: window.location.href,
      PATH: window.location.pathname,
      HOST: window.location.host,
      QUERY: window.location.search,
      HASH: window.location.hash,
      CONTENT_TYPE: document.contentType,
      CHARSET: document.characterSet
    }
  }
}

export class RFungeError extends Error {}

export class InterpreterStopped extends RFungeError {}
