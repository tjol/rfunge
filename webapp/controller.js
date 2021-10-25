import initRFunge, { BefungeInterpreter } from './rfunge_wasm/rfunge.js'

let wasmInitialized = false

let ticksPerCall = 1000

export class RFungeController {
  constructor (host) {
    this._host = host
    this._stopRequest = null
    this._inputBuffer = ''
    this._onInput = []
  }

  async init () {
    if (!wasmInitialized) {
      await initRFunge()
      wasmInitialized = true
    }
    this._interpreter = new BefungeInterpreter(this)
  }

  /******************************************************
    METHODS CALLED BY RUST
   ******************************************************/

  writeOutput (s) {
    this._host.ioRef.value.write(s)
  }

  warn (msg) {
    console.warn('RFunge warning: %s', msg)
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

  readInput () {
    return new Promise((resolve, reject) => {
      if (this._stopRequest != null) {
        reject(new InterpreterStopped())
      }
      if (this._inputBuffer !== '') {
        const result = this._inputBuffer
        this._inputBuffer = ''
        resolve(result)
      } else {
        const idx = this._onInput.length
        const inputCallback = () => {
          const result = this._inputBuffer
          this._inputBuffer = ''
          this._onInput.splice(idx, 1) // remove callback from array
          resolve(result)
        }
        this._onInput.push(inputCallback)
        this._inputStamp = true
      }
    })
  }

  /******************************************************
    METHODS CALLED BY THE JAVASCRIPT APP
   ******************************************************/

  run () {
    return new Promise((resolve, reject) => {
      const continueRunning = async () => {
        if (this._stopRequest != null) {
          let callback = this._stopRequest
          this._stopRequest = null
          reject(new InterpreterStopped())
          callback()
          return
        }
        this._inputStamp = false
        // Execute tickPerCall instructions and meausure how long it took
        const t0 = performance.now()
        const result = await this._interpreter.runLimitedAsync(ticksPerCall)
        const t1 = performance.now()
        const dt = t1 - t0

        if (result != null) {
          // We're done!
          resolve(result)
          return
        } else {
          // Continue running
          if (!this._inputStamp) {
            // only time iterations that did NOT wait for input
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
          }
          // go again - after returning control to the UI
          setTimeout(continueRunning, 0)
        }
      }
      this._mustStop = false
      // Need to use a setTimeout() pseudo-loop to avoid blocking the event loop
      continueRunning()
    })
  }

  async step () {
    return await this._interpreter.stepAsync()
  }

  stop () {
    return new Promise(resolve => {
      this.writeInput('') // EOF
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

  writeInput (s) {
    this._inputBuffer += s
    for (const callback of this._onInput) {
      callback()
    }
  }
}

export class RFungeError extends Error {}

export class InterpreterStopped extends RFungeError {}
