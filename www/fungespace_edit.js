export class FungeSpaceEdit {
    constructor(domId) {
        this._div = document.getElementById(domId)
        this._srcdiv = document.createElement("div")
        this._srcdiv.setAttribute("contentEditable", true)
        this._div.appendChild(this._srcdiv)

        this.setSrc(">2:*1..@")
    }

    setSrc(src, ips) {
        if (ips == null) { 
            ips = []
        }
        // turn it into HTML!
        let lines = src.replace(" ", "\xa0").split("\n")
        const escape = ((s) => s.replace(">", "&gt;").replace("<", "&lt;"))
        let ipGrid = []
        for (let ipLoc of ips) {
            let [x, y] = ipLoc
            if (ipGrid[y] === undefined) ipGrid[y] = [x]
            else ipGrid[y].push(x)
        }
        const maxY = Math.max(ipGrid.length, lines.length)
        for (let y = 0; y < maxY; ++y) {
            let ipIndices = [... new Set(ipGrid[y])]
            ipIndices.sort()
            let subStrings = []
            let rest = lines[y]
            if (rest === undefined) rest = ""
            let x0 = 0
            for (let x of ipIndices) {
                let relX = x - x0
                while (relX >= rest.length) rest = rest + "\xa0"
                subStrings.push(escape(rest.substring(0, relX)))
                subStrings.push("<span class=\"ip\">"
                    + escape(`${rest[relX]}`)
                    + "</span>")
                rest = rest.substring(relX + 1)
                x0 = x + 1
            }
            lines[y] = subStrings.join("") + escape(rest)
        }

        const htmlSrc = lines.map(l => `<div>${l}</div>`).join("")

        this._srcdiv.innerHTML = htmlSrc;
    }

    getSrc() {
        let src = "";
        this._srcdiv.childNodes.forEach((node, idx) => {
            if (node.nodeValue !== null) {
                src += node.nodeValue;
            } else if (node.nodeType === Node.ELEMENT_NODE) {
                if (node.tagName === "DIV") {
                    // This is a new line
                    if (idx != 0) {
                        src += "\n"
                    }
                    src += node.textContent
                } else if (node.tagName === "BR") {
                    src += "\n"
                }
            }
        })
        // Get rid of NBSP
        return src.replace("\u00a0", " ")
    }
}

