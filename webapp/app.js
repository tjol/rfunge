import { createRef, ref } from 'lit/directives/ref.js'
import { html, css, LitElement } from 'lit'
import { RFungeController, RFungeError } from './controller.js'
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
    if (this.mode == RFungeMode.EDIT) {
      buttonbar = html`
        <nav>
          <input type="button" @click="${this._run}" value="Run" />
        </nav>
      `
    }
    let outputArea = html`
      <output-area ${ref(this.stdout)}></output-area>
    `
    return html`
      ${editor}${buttonbar}${outputArea}
    `
  }

  async _run () {
    this.mode = RFungeError.DEBUG
    this._controller.reset()
    let src = this.editor.value.getSrc()
    this._controller.setSrc(src)
    await this._controller.run()
    this.editor.value.src = this._controller.getSrc()
    this.mode = RFungeMode.EDIT
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
