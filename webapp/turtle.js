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
import { ref, createRef } from 'lit/directives/ref.js'

export class TurtleWindow extends LitElement {
  _canvasRef = createRef()

  constructor () {
    super()
  }

  render () {
    return html`
      <div>
        <canvas ${ref(this._canvasRef)}></canvas><img src="" class="stretch" />
      </div>
    `
  }

  static styles = css`
    .stretch {
      width: 100%;
      visibility: hidden;
      height: 0;
    }
  `
}
window.customElements.define('rfunge-turt-window', TurtleWindow)

function cssColour (colour) {
  return `rgb(${colour.r}, ${colour.g}, ${colour.b})`
}

export class TurtleDisplay {
  constructor (app) {
    this._app = app
    this._redraw = false
  }

  display (show) {
    this._app.turtActive = show
  }

  isDisplayVisible () {
    return this._app.turtActive
  }

  _bounds (lines, dots) {
    if (lines.length === 0 && dots.length === 0) {
      return [0, 0, 0, 0]
    }
    let top = null
    let left = null
    let bottom = null
    let right = null
    for (const l of lines) {
      const minX = Math.min(l.from.x, l.to.x)
      const minY = Math.min(l.from.y, l.to.y)
      const maxX = Math.max(l.from.x, l.to.x)
      const maxY = Math.max(l.from.y, l.to.y)
      top = top == null ? minY : Math.min(minY, top)
      left = left == null ? minX : Math.min(minX, left)
      bottom = bottom == null ? maxY : Math.max(maxY, bottom)
      right = right == null ? maxX : Math.max(maxX, right)
    }
    for (const d of dots) {
      top = top == null ? d.pos.y : Math.min(d.pos.y, top)
      left = left == null ? d.pos.x : Math.min(d.pos.x, left)
      bottom = bottom == null ? d.pos.y : Math.max(d.pos.y, bottom)
      right = right == null ? d.pos.x : Math.max(d.pos.x, right)
    }
    return [left, top, right, bottom]
  }

  print (background, lines, dots) {
    // create a new canvas
    const canvas = document.createElement('canvas')
    // minimum size
    canvas.width = 10
    canvas.height = 10
    // Draw, expanding the canvas
    this._drawToCanvas(canvas, false, background, lines, dots)
    // Get the image
    const url = canvas.toDataURL()
    const linkElem = document.createElement('a')
    linkElem.href = url
    linkElem.download = 'turtle.png'
    document.body.appendChild(linkElem)
    linkElem.click()
    document.body.removeChild(linkElem)
  }

  _drawToCanvas (canvas, rescale = true, background, lines, dots) {
    const [left, top, right, bottom] = this._bounds(lines, dots)
    let { width, height } = canvas
    // Do have have to scale to fit?
    const imageWidth = right - left + 8
    const imageHeight = bottom - top + 8
    let scale = 1
    if (imageWidth > width) {
      if (rescale) {
        scale = width / imageWidth
      } else {
        width = imageWidth
        canvas.width = width
      }
    } else if (rescale) {
      while (imageWidth * scale < width / 10 && scale < 16) {
        scale *= 2
      }
    }
    // Make sure we have enough vertical space
    height = Math.max(height, scale * imageHeight)
    canvas.height = height
    // centre the image
    const offsetX = width / 2 - (scale * (right + left)) / 2 + 4 + 0.5
    // align it to the top
    const offsetY = -scale * top + 4 + 0.5
    // Draw
    const ctx = canvas.getContext('2d')
    // Step 1: fill in the background
    if (background == null) {
      ctx.clearRect(0, 0, width, height)
    } else {
      ctx.fillStyle = cssColour(background)
      ctx.fillRect(0, 0, width, height)
    }
    // Step 2: draw all the lines
    for (const line of lines) {
      ctx.beginPath()
      ctx.strokeStyle = cssColour(line.colour)
      ctx.moveTo(offsetX + scale * line.from.x, offsetY + scale * line.from.y)
      ctx.lineTo(offsetX + scale * line.to.x, offsetY + scale * line.to.y)
      ctx.lineWidth = scale
      ctx.lineCap = 'square'
      ctx.stroke()
    }
    // Step 3: draw all the dots
    for (const dot of dots) {
      ctx.fillStyle = cssColour(dot.colour)
      ctx.fillRect(
        offsetX + scale * (dot.pos.x - 0.5),
        offsetY + scale * (dot.pos.y - 0.5),
        scale,
        scale
      )
    }
  }

  draw (background, lines, dots) {
    this._background = background
    this._lines = lines
    this._dots = dots
    this._redraw = true
    setTimeout(() => {
      if (
        this._app.turtActive &&
        this._app.turtWindowRef.value != null &&
        this._redraw
      ) {
        const turtWnd = this._app.turtWindowRef.value
        const canvas = turtWnd._canvasRef.value
        if (canvas != null) {
          // Resize the canvas
          canvas.width = canvas.parentNode.clientWidth
          // Draw the image
          this._drawToCanvas(
            canvas,
            true,
            this._background,
            this._lines,
            this._dots
          )
        }
        this._redraw = false
      }
    }, 0)
  }
}
