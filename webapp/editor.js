import { html, css, LitElement } from 'lit'
import { ref, createRef } from 'lit/directives/ref.js'
import { RFungeMode } from './rfunge-common.js'

export class RFungeEditor extends LitElement {
  _textArea = createRef()

  static properties = {
    mode: { type: Number },
    src: { type: String },
    srcLines: { type: Array },
    cursors: { type: Array }
  }

  constructor () {
    super()
    this.mode = RFungeMode.EDIT
    this.src = ''
    this.srdLines = []
    this.cursors = []
  }

  render () {
    switch (this.mode) {
      case RFungeMode.EDIT:
        return this.renderEditor()
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
      <textarea ${ref(this._textArea)} .value="${this.src}"></textarea>
    `
  }
  renderDebugger () {
    return html`
    <div class="debug-src">
    ${this.srcLines.map(line => html`<p>${
      Array.from(line).map(c => {
        if (c == ' ') return html`<span class="cell space">\xa0</span>`
        else if (c.match(/\p{Z}|\p{C}/u)) return html`<span class="cell as-number">${c.codePointAt(0)}</span>`
        else return html`<span class="cell">${c}</span>`
      })
    }</p>`)}
    </div>
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

  static styles = css`
  .debug-src {
    font-family: monospace;
    font-size: 1.1em;
  }
  .debug-src p {
    margin: 0;
    padding: 0;
    margin-bottom: 0.2em;
  }
  .cell {
    display: inline-block;
    width: 1em;
    text-align: center;
  }
`
}
window.customElements.define('rfunge-editor', RFungeEditor)
