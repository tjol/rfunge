// lib
import { html, css, LitElement } from 'lit'
import { createRef, ref } from 'lit/directives/ref.js'

// project (js)
import { InterpreterStopped, RFungeController } from './controller.js'
import { RFungeMode, COMMON_STYLES } from './rfunge-common.js'

// project (web components)
import { RFungeEditor } from './editor.js'
import { StackWindow } from './stack-window.js'
import { IOWindow } from './io-window.js'

export class RFungeGui extends LitElement {
  editorRef = createRef()
  ioRef = createRef()
  stackWindowRef = createRef()

  static properties = {
    mode: { type: Number },
    heading: { type: String }
  }

  constructor () {
    super()
    this.mode = RFungeMode.INACTIVE
    this._controller = new RFungeController(this)
    this._controller.init().then(
      () => {
        this.mode = RFungeMode.EDIT
      },
      () => {
        alert('WASM error!')
      }
    )
  }

  render () {
    let editor = html`
      <rfunge-editor ${ref(this.editorRef)} mode="${this.mode}"></rfunge-editor>
    `
    let buttonbar = ''
    switch (this.mode) {
      case RFungeMode.EDIT:
        buttonbar = html`
          <nav>
            <input type="button" @click="${this._run}" value="Run" />
            <input
              type="button"
              @click="${this._startDebugger}"
              value="Debug"
            />
          </nav>
        `
        break
      case RFungeMode.RUN:
        buttonbar = html`
          <nav>
            <input type="button" @click="${this._stop}" value="Stop" />
          </nav>
        `
        break
      case RFungeMode.DEBUG:
        buttonbar = html`
          <nav>
            <input type="button" @click="${this._step}" value="Step" />
            <input
              type="button"
              @click="${this._continueRunning}"
              value="Continue"
            />
            <input type="button" @click="${this._stopDebugger}" value="Abort" />
          </nav>
        `
        break
      case RFungeMode.DEBUG_FINISHED:
        buttonbar = html`
          <nav>
            <input
              type="button"
              @click="${this._closeDebugger}"
              value="Close Debugger"
            />
          </nav>
        `
        break
    }
    let outputArea = html`
      <rfunge-io-window
        ${ref(this.ioRef)}
        @rfunge-input="${this._onInput}"
      ></rfunge-io-window>
    `
    let stackWindow = this.mode == RFungeMode.DEBUG ? html`
      <rfunge-stack-window
        ${ref(this.stackWindowRef)}
        mode="${this.mode}"
      ></rfunge-stack-window>
    ` : ""

    return html`
      <div id="core">
      ${this.heading !== "" ? html`<h1>${this.heading}</h1>` : ""}
      ${editor}${buttonbar}${outputArea}
      </div>
      ${stackWindow}
    `
  }

  static styles = css` ${COMMON_STYLES}
    nav {
      margin: 0;
    }
    :host {
      display: flex;
      flex-direction: column;
    }
    :host > * {
      padding: 0 1rem;
    }
    rfunge-stack-window {
      flex-grow: 1;
    }

    @media only screen and (min-width: 1280px) {
      :host {
        flex-direction: row;
      }
      div#core {
        flex-grow: 1;
      }
      rfunge-stack-window {
        flex-grow: 0;
        width: 400px;
      }
    }
  `

  async _run () {
    this.mode = RFungeMode.RUN
    this._controller.reset()
    let src = this.editorRef.value.getSrc()
    this._controller.setSrc(src)
    this.editorRef.value.srcLines = this._controller.getSrcLines()
    try {
      let result = await this._controller.run()
      this._done(result)
    } catch (e) {
      if (e instanceof InterpreterStopped) {
        console.log('Interpreter stopped at user request')
      } else {
        console.warn(`An error occurred: ${e}`)
      }
    } finally {
      this.editorRef.value.src = src // replace the original source code
      this.mode = RFungeMode.EDIT
    }
  }

  _done (result) {
    this.ioRef.value.writeLine(`\nFinished with code ${result}`)
  }

  _startDebugger () {
    this.mode = RFungeMode.DEBUG
    this._controller.reset()
    let src = this.editorRef.value.getSrc()
    this._origSrc = src
    this._controller.setSrc(src)
    this.editorRef.value.srcLines = this._controller.getSrcLines()
    this.editorRef.value.cursors = this._controller.getCursors()
  }

  async _continueRunning () {
    try {
      this._done(await this._controller.run())
    } catch (e) {
      if (e instanceof InterpreterStopped) {
        console.log('Interpreter stopped at user request')
      } else {
        console.warn(`An error occurred: ${e}`)
      }
    } finally {
      this.mode = RFungeMode.DEBUG_FINISHED
      this._syncDebuggerState()
    }
  }

  async _step () {
    let result = await this._controller.step()
    if (result != null) {
      // process ended
      this._done(result)
      this.mode = RFungeMode.DEBUG_FINISHED
    }
    this._syncDebuggerState()
  }

  _syncDebuggerState () {
    this.editorRef.value.srcLines = this._controller.getSrcLines()
    this.editorRef.value.cursors = this._controller.getCursors()
    this.stackWindowRef.value.stacks = this._controller.getStacks()
  }

  _stopDebugger () {
    // stop if currently running
    this._controller.stop()
    // free up memory, maybe
    this._controller.reset()
    // reset UI
    this._closeDebugger()
  }

  _closeDebugger () {
    this.editorRef.value.src = this._origSrc // replace the original source code
    this.mode = RFungeMode.EDIT
  }

  _stop () {
    this._controller.stop()
  }

  _onInput (ev) {
    this._controller.writeInput(ev.detail.value)
  }
}
window.customElements.define('rfunge-app', RFungeGui)
