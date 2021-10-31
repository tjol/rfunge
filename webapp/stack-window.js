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
        <h2>Stack</h2>
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
                              ${Array.from(stack)
                                .reverse()
                                .map(
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

    h2 {
      font-size: 1.4em;
      text-transform: uppercase;
    }

    h5 {
      font-size: 1em;
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
      display: flex;
      flex-direction: column;
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
      ul.stack-stack {
        flex-direction: row;
      }
    }
  `
}
window.customElements.define('rfunge-stack-window', StackWindow)
