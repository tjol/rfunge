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
      case RFungeMode.DEBUG_FINISHED:
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
    // Transpose (in a manner of speaking) the information about IP locations
    let ipPositionClasses = {}
    this.cursors.forEach((ipInfo, ipIndex) => {
      const loc = ipInfo.location
      if (!(loc in ipPositionClasses)) ipPositionClasses[loc] = []
      ipPositionClasses[loc].push('ip-location')
      ipPositionClasses[loc].push(`ip-${ipIndex}-location`)
      const nextLoc = ipInfo.projectedLocation
      if (!(nextLoc in ipPositionClasses)) ipPositionClasses[nextLoc] = []
      ipPositionClasses[nextLoc].push('ip-next-location')
      ipPositionClasses[nextLoc].push(`ip-${ipIndex}-next-location`)
    })

    // render the content
    return html`
      <div class="debug-src">
        ${this.srcLines.map(
          (line, y) =>
            html`
              <p>
                ${Array.from(line).map((c, x) => {
                  let classes = ["cell"]
                  const pos = [x,y]
                  if (pos in ipPositionClasses) {
                    classes.push(...ipPositionClasses[pos])
                  }
                  if (c == ' ')
                    return html`
                      <span class="${classes.join(" ")} space"> </span>
                    `
                  else if (c.match(/\p{Z}|\p{C}/u))
                    return html`
                      <span class="${classes.join(" ")} as-number">${c.codePointAt(0)}</span>
                    `
                  else
                    return html`
                      <span class="${classes.join(" ")}">${c}</span>
                    `
                })}
              </p>
            `
        )}
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
    textarea, .debug-src {
      font-size: 1.1rem;
      font-family: monospace;
      font-family: var(--code-font);
      width: 100%;
      box-sizing: border-box;
      margin: 1rem 0;
      padding: 0.5rem;
      border: 1px solid #aaa;
      min-height: 25rem;
      background-color: inherit;
      color: inherit;
    }
    textarea {
      letter-spacing: 0.5em;
    }
    .debug-src {
      overflow-x: auto;
    }
    .debug-src p {
      margin: 0;
      padding: 0;
      margin-bottom: 0.2rem;
    }
    .cell {
      display: inline-block;
      width: 1rem;
      text-align: center;
    }
    .cell.as-number {
      font-size: 0.5rem;
      word-break: break-all;
    }
    .ip-next-location {
      background-color: pink;
      background-color: var(--projected-cursor-background);
      color: var(--projected-cursor-color);
    }
    .ip-location {
      background-color: lavenderblush;
      background-color: var(--last-cursor-background);
      color: var(--last-cursor-color);
    }
  `
}
window.customElements.define('rfunge-editor', RFungeEditor)
