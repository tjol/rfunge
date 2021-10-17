import { html, css, LitElement } from 'lit'
import { ref, createRef } from 'lit/directives/ref.js'
import { RFungeMode } from './rfunge-common.js'

export class RFungeEditor extends LitElement {
  _textArea = createRef()

  static properties = {
    mode: { type: Number },
    src: { type: String },
    cursors: { type: Array }
  }

  constructor () {
    super()
    this.mode = RFungeMode.EDIT
    this.src = ''
    this.cursors = []
  }

  render () {
    switch (this.mode) {
      case RFungeMode.EDIT:
        return this.renderEditor()
        break
      case RFungeMode.DEBUG:
      case RFungeMode.RUN:
        return this.renderDebugger()
      case RFungeMode.INACTIVE:
        return ''
      default:
        return html`
          ERROR
        `
    }
  }
  renderEditor () {
    return html`
      <textarea ${ref(this._textArea)}>${this.src}</textarea>
    `
  }
  renderDebugger () {
    return html`
      <p>Debugger goes here</p>
    `
  }

  getSrc () {
    switch (this.mode) {
      case RFungeMode.EDIT:
        return this._textArea.value.value
      default:
        return this.src
    }
  }
}
window.customElements.define('rfunge-editor', RFungeEditor)

