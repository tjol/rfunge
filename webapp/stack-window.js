import { html, css, LitElement } from 'lit'
import { RFungeMode } from './rfunge-common'

export class StackWindow extends LitElement {
  static properties = {
    mode: { type: Number },
    stacks: { type: Array }
  }

  constructor () {
    super()
    this.mode = RFungeMode.INACTIVE
    this.stacks = []
  }

  render () {
    if (this.mode === RFungeMode.DEBUG) {
      return html`
        <ul class="ip-list">
          ${this.stacks.map(
            (stackStack, ipIndex) =>
              html`
                <li>
                  <h5>IP ${ipIndex}</h5>
                  <ul class="stack-stack">
                    ${stackStack.map(
                      stack =>
                        html`
                          <li>
                            <ul class="stack">
                              ${Array.from(stack).reverse().map(
                                stackElem => html`
                                  <li>${stackElem}</li>
                                `
                              )}
                            </ul>
                          </li>
                        `
                    )}
                  </ul>
                </li>
              `
          )}
        </ul>
      `
    } else {
      return html``
    }
  }
}
window.customElements.define('rfunge-stack-window', StackWindow)
