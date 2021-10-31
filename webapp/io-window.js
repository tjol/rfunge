/*
rfunge – a Funge-98 interpreter
Copyright © 2021 Thomas Jollans

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU Affero General Public License as
published by the Free Software Foundation, either version 3 of the
License, or (at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
GNU Affero General Public License for more details.

You should have received a copy of the GNU Affero General Public License
along with this program. If not, see <https://www.gnu.org/licenses/>.
*/

import { LitElement, html, css } from 'lit'
import { createRef, ref } from 'lit/directives/ref.js'
import { COMMON_STYLES } from './rfunge-common'

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
    ${COMMON_STYLES}
    p {
      font-family: monospace;
      font-family: var(--code-font);
      white-space: pre-wrap;
    }

    input[type='text'] {
      font-family: var(--code-font);
      width: 20em;
      max-width: calc(100vw - 6.5rem);
    }
  `
}
window.customElements.define('rfunge-io-window', IOWindow)
