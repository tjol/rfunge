import initRFunge, { BefungeInterpreter } from './rfunge_wasm/rfunge.js'

let wasmInitialized = false

let ticksPerCall = 1000

export class RFungeController {
  constructor (host) {
    this._host = host
    this._stopRequest = null
  }

  async init () {
    if (!wasmInitialized) {
      await initRFunge()
      wasmInitialized = true
    }
    this._interpreter = new BefungeInterpreter(this)
  }

  writeOutput (s) {
    this._host.stdoutRef.value.write(s)
  }

  warn (msg) {
    console.warn('RFunge warning: %s', msg)
  }

  run () {
    return new Promise((resolve, reject) => {
      const continueRunning = () => {
        if (this._stopRequest != null) {
          let callback = this._stopRequest
          this._stopRequest = null
          reject(new InterpreterStopped())
          callback()
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
          const timeFactor = (dt + 1) / 100
          if (timeFactor > 2) {
            // too slow
            ticksPerCall = Math.floor(ticksPerCall / timeFactor)
            console.log(`adjusting to ${ticksPerCall} ticks per iteration`)
          } else if (timeFactor < 0.5) {
            // too fast
            ticksPerCall = Math.floor(ticksPerCall / timeFactor)

            // prevent overflow on supercomputers (or with buggy WASM...)
            if (ticksPerCall > 1 << 30) ticksPerCall = 1 << 30

            console.log(`adjusting to ${ticksPerCall} ticks per iteration`)
          }
          // go again
          setTimeout(continueRunning, 0)
        }
      }
      this._mustStop = false
      continueRunning()
    })
  }

  step() {
    return this._interpreter.step()
  }

  stop () {
    return new Promise((resolve, _) => {
      this._stopRequest = resolve
    })
  }

  reset () {
    this._interpreter.close()
    this._interpreter = new BefungeInterpreter(this)
  }

  setSrc (src) {
    this._interpreter.loadSrc(src)
  }

  getSrc () {
    return this._interpreter.getSrc()
  }

  getSrcLines () {
    return this._interpreter.getSrcLines()
  }

  getCursors () {
    const ipCount = this._interpreter.ipCount
    let cursors = []
    for (let i = 0; i < ipCount; ++i) {
      cursors.push({
        location: this._interpreter.ipLocation(i),
        delta: this._interpreter.ipDelta(i),
        projectedLocation: this._interpreter.projectedIpLocation(i)
      })
    }
    return cursors
  }

  getStacks () {
    const ipCount = this._interpreter.ipCount
    let stackStackStack = []
    for (let i = 0; i < ipCount; ++i) {
      const stackCount = this._interpreter.stackCount(i)
      let stackStack = []
      for (let j = 0; j < stackCount; ++j) {
        stackStack.push(this._interpreter.getStack(i, j))
      }
      stackStackStack.push(stackStack)
    }
    return stackStackStack
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
