import { html, css, LitElement } from 'lit'
import { COMMON_STYLES, RFungeMode } from './rfunge-common'

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

  static styles = css`
    ${COMMON_STYLES}

    :host {
      background-color: var(--other-background-color);
      color: var(--other-text-color);
      width: 100%;
      margin: 0;
      margin-top: 2em;
      padding: 0 1em;
    }

    * {
      background-color: inherit;
      color: inherit;
    }

    li {
      list-style-type: none;
      padding: 0;
      margin: 0;
    }

    ul.stack-stack {
      margin: 0;
      padding: 0;
    }

    ul.stack {
      margin: 0;
      padding: 0;
      display: block;
    }

    ul.stack > li {
      display: inline-block;
      font-family: var(--code-font);
      margin: 0 1em;
    }

    @media only screen and (min-width: 1280px) {
      :host {
        margin-top: 0;
        margin-left: 1em;
      }
      ul.stack > li {
        display: block;
      }
    }
  `
}
window.customElements.define('rfunge-stack-window', StackWindow)
