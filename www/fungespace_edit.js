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


export class FungeSpaceEdit {
    constructor(domId) {
        this._div = document.getElementById(domId)
        this._srcdiv = document.createElement("div")
        this._div.appendChild(this._srcdiv)

        this.setSrc("")
        this.enableEditing()
    }

    setSrc(src, ips) {
        if (ips == null) { 
            ips = []
        }
        // turn it into HTML!
        let lines
        if (Array.isArray(src)) {
            lines = src
        } else {
            lines = src.split("\n")
        }
        const escape = ((s) => Array.from(s).map(c => {
            switch (c) {
                case "&": return "&amp;"
                case ">": return "&gt;"
                case "<": return  "&lt;"
                case "\xa0": return "␣"
                case " ": return "\xa0"
                case "\x7f": return "␡"
                default:
                    if (c.codePointAt(0) < 0x20) // control characters
                        return String.fromCodePoint(0x2400 + c.codePointAt(0))
                    else return c
            }
        }).join(""))
        let ipGrid = []
        for (let ipLoc of ips) {
            let [x, y] = ipLoc
            if (ipGrid[y] === undefined) ipGrid[y] = [x]
            else ipGrid[y].push(x)
        }
        const maxY = Math.max(ipGrid.length, lines.length)
        for (let y = 0; y < maxY; ++y) {
            let ipIndices = [... new Set(ipGrid[y])]
            ipIndices.sort((a, b) => a - b) // numeric sort needs custom cmp
            let subStrings = []
            let rest = lines[y]
            if (rest === undefined) rest = ""
            let x0 = 0
            for (let x of ipIndices) {
                let relX = x - x0
                while (relX >= rest.length) rest = rest + " "
                subStrings.push(escape(rest.substring(0, relX)))
                subStrings.push("<span class=\"ip\">"
                    + escape(`${rest[relX]}`)
                    + "</span>")
                rest = rest.substring(relX + 1)
                x0 = x + 1
            }
            lines[y] = subStrings.join("") + escape(rest)
        }

        if (lines.length === 1 && lines[0] === "") {
            lines = [];
        }

        const htmlSrc = lines.map(l => `<div>${l}\xa0</div>`).join("")

        this._srcdiv.innerHTML = htmlSrc;
    }

    getSrc() {
        let src = elemToString(this._srcdiv)
        // Get rid of NBSP
        return src.replaceAll("\u00a0", " ")
    }

    enableEditing() {
        this._srcdiv.contentEditable = true
    }

    disableEditing() {
        this._srcdiv.contentEditable = false
    }
}

function elemToString(elem) {
    let src = ""
    elem.childNodes.forEach(node => {
        if (node.nodeValue !== null) {
            src += node.nodeValue.replaceAll("\n", "");
        } else if (node.nodeType === Node.ELEMENT_NODE) {
            src += elemToString(node)
        }
    })
    if (window.getComputedStyle(elem).display === "block" || elem.tagName === "BR") {
        src += "\n"
    }
    return src
}

export default FungeSpaceEdit