import { LitElement, html, css } from 'lit'
import { createRef, ref } from 'lit/directives/ref.js'

export class IOWindow extends LitElement {
  inputRef = createRef()

  static properties = {
    text: { type: String }
  }

  constructor () {
    super()
    this.text = ''
  }

  render () {
    return html`
      <div>
        <p>${this.text}</p>
        <form @submit="${this._submit}">
          <input type="text" @keyup="${this._onKeyUp}" ${ref(this.inputRef)} />
          <input type="submit" value="⏎" />
        </form>
      </div>
    `
  }

  write (s) {
    this.text += s
  }

  writeLine (ln) {
    this.write(`${ln}\n`)
  }

  _submit (submitEvent) {
    // don't submit the form the old-fashioned way
    submitEvent.preventDefault()

    const inputElem = this.inputRef.value
    const line = inputElem.value
    inputElem.value = ''
    const ev = new CustomEvent('rfunge-input', {
      detail: { value: `${line}\n` },
      bubbles: true,
      composed: true
    })
    this.dispatchEvent(ev)

    return true
  }

  static styles = css`
    p {
      font-family: monospace;
      white-space: pre-wrap;
    }
  `
}
window.customElements.define('rfunge-io-window', IOWindow)
