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

class RGB {
  constructor (r, g, b) {
    this.r = r
    this.g = g
    this.b = b
  }

  get cssColour () {
    return `rgb(${this.r}, ${this.g}, ${this.b})`
  }
}

export class TurtleState {
  constructor (app) {
    this._app = app
    this._heading = 0
    this._x = 0
    this._y = 0
    this._penDown = false
    this._lines = []
    this._dots = []
    this._colour = new RGB(0, 0, 0)
    this._backgroundColour = null
    this._haveDrawn = false
  }

  turnLeft (degrees) {
    this._heading -= degrees
  }
  setHeading (degrees) {
    this._heading = degrees
  }
  getHeading () {
    return this._heading
  }
  setPen (down) {
    if (this._penDown && !down) {
      // Pen going up, was down
      if (!this._haveDrawn) {
        // Add a dot
        this._dots.push({pos: [this._x, this._y], colour: this._colour});
      }
    }
    this._penDown = down
    if (!this._penDown) {
      this._haveDrawn = false
    }
  }
  isPenDown () {
    return this._penDown
  }
  forward (pixels) {
    // calculate the new position
    const heading_rad = (this._heading / 180) * Math.PI
    const destX = this._x + pixels * Math.cos(heading_rad)
    const destY = this._y + pixels * Math.sin(heading_rad)
    if (this._penDown) {
      this._lines.push({
        from: [this._x, this._y],
        to: [destX, destY],
        colour: this._colour
      })
      this._haveDrawn = true
      this._redraw()
    }
    this._x = destX
    this._y = destY
  }
  setColour (r, g, b) {
    this._colour = new RGB(r, g, b)
  }
  clearWithColour (r, g, b) {
    this._backgroundColour = new RGB(r, g, b)
    this._lines = []
    this._redraw()
  }
  display (show) {
    this._app.turtActive = show
    setTimeout(() => this._redraw(), 0)
  }
  teleport (x, y) {
    this._x = x
    this._y = y
  }
  position () {
    return [this._x, this._y]
  }
  bounds () {
    if (this._lines.length === 0 && this._dots.length === 0) {
      return [0, 0, 0, 0]
    }
    let top = null
    let left = null
    let bottom = null
    let right = null
    for (const l of this._lines) {
      const minX = Math.min(l.from[0], l.to[0])
      const minY = Math.min(l.from[1], l.to[1])
      const maxX = Math.max(l.from[0], l.to[0])
      const maxY = Math.max(l.from[1], l.to[1])
      top = top == null ? minY : Math.min(minY, top)
      left = left == null ? minX : Math.min(minX, left)
      bottom = bottom == null ? maxY : Math.max(maxY, bottom)
      right = right == null ? maxX : Math.max(maxX, right)
    }
    for (const d of this._dots) {
      top = top == null ? d.pos[1] : Math.min(d.pos[1], top)
      left = left == null ? d.pos[0] : Math.min(d.pos[0], left)
      bottom = bottom == null ? d.pos[1] : Math.max(d.pos[1], bottom)
      right = right == null ? d.pos[0] : Math.max(d.pos[0], right)
    }
    return [left, top, right, bottom]
  }

  print () {
    // create a new canvas
    const canvas = document.createElement('canvas')
    // minimum size
    canvas.width = 10
    canvas.height = 10
    // Draw, expanding the canvas
    this._drawToCanvas(canvas, false)
    // Get the image
    const url = canvas.toDataURL()
    const linkElem = document.createElement('a')
    linkElem.href = url
    linkElem.download = 'turtle.png'
    document.body.appendChild(linkElem)
    linkElem.click()
    document.body.removeChild(linkElem)
  }

  _drawToCanvas (canvas, rescale = true) {
    const [left, top, right, bottom] = this.bounds()
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
    if (this._backgroundColour == null) {
      ctx.clearRect(0, 0, width, height)
    } else {
      ctx.fillStyle = this._backgroundColour.cssColour
      ctx.fillRect(0, 0, width, height)
    }
    // Step 2: draw all the lines
    for (const line of this._lines) {
      ctx.beginPath()
      ctx.strokeStyle = line.colour.cssColour
      ctx.moveTo(offsetX +  scale * line.from[0], offsetY + scale * line.from[1])
      ctx.lineTo(offsetX + scale * line.to[0], offsetY + scale * line.to[1])
      ctx.lineWidth = scale
      ctx.lineCap = "square"
      ctx.stroke()
    }
    // Step 3: draw all the dots
    for (const dot of this._dots) {
      ctx.fillStyle = dot.colour.cssColour
      ctx.fillRect(offsetX + scale * (dot.pos[0] - 0.5),
                   offsetY + scale * (dot.pos[1] - 0.5),
                   scale, scale)
    }
    // Step 4: if the pen is down, maybe draw a dot
    if (this._penDown && !this._haveDrawn)
    {
      ctx.fillStyle = this._colour.cssColour
      ctx.fillRect(offsetX + scale * (this._x - 0.5),
                   offsetY + scale * (this._y - 0.5),
                   scale, scale)
    }
  }

  _redraw () {
    if (this._app.turtActive && this._app.turtWindowRef.value != null) {
      const turtWnd = this._app.turtWindowRef.value
      const canvas = turtWnd._canvasRef.value
      if (canvas != null) {
        // Resize the canvas
        canvas.width = canvas.parentNode.clientWidth
        // Draw the image
        this._drawToCanvas(canvas, true)
      }
    }
  }
}
