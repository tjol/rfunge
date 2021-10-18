import { createRef, ref } from 'lit/directives/ref.js'
import { html, css, LitElement } from 'lit'
import {
  InterpreterStopped,
  RFungeController,
  RFungeError
} from './controller.js'
import { RFungeEditor } from './editor.js'
import { RFungeMode } from './rfunge-common.js'

export class RFungeGui extends LitElement {
  editor = createRef()
  stdout = createRef()

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
      <rfunge-editor ${ref(this.editor)} mode="${this.mode}"></rfunge-editor>
    `
    let buttonbar = ''
    switch (this.mode) {
      case RFungeMode.EDIT:
        buttonbar = html`
          <nav>
            <input type="button" @click="${this._run}" value="Run" />
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
    }
    let outputArea = html`
      <output-area ${ref(this.stdout)}></output-area>
    `
    return html`
      ${editor}${buttonbar}${outputArea}
    `
  }

  async _run () {
    this.mode = RFungeMode.RUN
    this._controller.reset()
    let src = this.editor.value.getSrc()
    this._controller.setSrc(src)
    try {
      await this._controller.run()
    } catch (e) {
      if (e instanceof InterpreterStopped) {
        console.log('Interpreter stopped at user request')
      } else {
        console.warn(`An error occurred: ${e}`)
      }
    } finally {
      this.editor.value.src = this._controller.getSrc()
      this.mode = RFungeMode.EDIT
    }
  }

  async _stop () {
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
