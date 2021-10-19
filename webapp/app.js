// lib
import { createRef, ref } from 'lit/directives/ref.js'
import { html, css, LitElement } from 'lit'

// project (js)
import { InterpreterStopped, RFungeController } from './controller.js'
import { RFungeMode } from './rfunge-common.js'

// project (web components)
import { RFungeEditor } from './editor.js'
import { StackWindow } from './stack-window.js'

export class RFungeGui extends LitElement {
  editorRef = createRef()
  stdoutRef = createRef()
  stackWindowRef = createRef()

  static properties = {
    mode: { type: Number }
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
      <output-area ${ref(this.stdoutRef)}></output-area>
    `
    let stackWindow = html`
      <rfunge-stack-window
        ${ref(this.stackWindowRef)}
        mode="${this.mode}"
      ></rfunge-stack-window>
    `
    return html`
      ${editor}${buttonbar}${outputArea}${stackWindow}
    `
  }

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
    this.stdoutRef.value.writeLine(`\nFinished with code ${result}`)
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

  _step () {
    let result = this._controller.step()
    if (result != null) {
      // process ended
      this._done(result)
      this.mode = RFungeMode.DEBUG_FINISHED
    }
    // update state
    this.editorRef.value.srcLines = this._controller.getSrcLines()
    this.editorRef.value.cursors = this._controller.getCursors()
    this.stackWindowRef.value.stacks = this._controller.getStacks()
  }

  _stopDebugger () {
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
}
window.customElements.define('rfunge-app', RFungeGui)

class OutputArea extends LitElement {
  static properties = {
    text: { type: String }
  }

  constructor () {
    super()
    this.text = ''
  }

  render () {
    return html`
      <p>${this.text}</p>
    `
  }

  write (s) {
    this.text += s
  }

  writeLine (ln) {
    this.write(`${ln}\n`)
  }

  static styles = css`
    p {
      font-family: monospace;
      white-space: pre-wrap;
    }
  `
}
window.customElements.define('output-area', OutputArea)
