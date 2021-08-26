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

import initWasm, * as rfunge from "./wasm_pkg/rfunge.js"
import FungeSpaceEdit from "./fungespace_edit.js"

async function initialize() {
    await initWasm()
    let editor = new FungeSpaceEdit("fungespace")
    let interpreter = new rfunge.BefungeInterpreter()
    interpreter.init()
    let finished = false
    let origSrc = null
    let stop = false

    let ticks_per_call = 1000

    document.getElementById("run-btn").onclick = () => {
        if (finished) return
        if (origSrc == null) {
            const src = editor.getSrc()
            origSrc = src
            interpreter.replaceSrc(src)
            editor.disableEditing()
        }
        document.getElementById("step-btn").disabled = true
        document.getElementById("run-btn").disabled = true
        document.getElementById("reset-btn").disabled = true
        document.getElementById("stop-btn").disabled = false

        function continueRunning() {
            if (stop) {
                finished = true
                editor.setSrc(interpreter.getSrcLines())
                document.getElementById("returncode-info").innerText = "Aborted by user"
                document.getElementById("reset-btn").disabled = false
                document.getElementById("stop-btn").disabled = true
                return
            }
            // Return control to the main event loop after every step
            // so we don't hang the browser
            const t0 = performance.now()
            const result = interpreter.run_limited(ticks_per_call)
            const t1 = performance.now()
            const dt = t1 - t0

            if (result != null) {
                finished = true
                editor.setSrc(interpreter.getSrcLines())
                document.getElementById("returncode-info").innerText = `Exited with status ${result}`
                document.getElementById("reset-btn").disabled = false
                document.getElementById("stop-btn").disabled = true
            } else {
                // target: 100ms per interval
                const time_factor = dt / 100
                if (time_factor > 2) {
                    // too slow
                    ticks_per_call = Math.floor(ticks_per_call / time_factor)
                    console.log(`adjusting to ${ticks_per_call} ticks per iteration`)
                } else if (time_factor < 0.5) {
                    // too fast
                    ticks_per_call = Math.floor(ticks_per_call / time_factor)
                    console.log(`adjusting to ${ticks_per_call} ticks per iteration`)
                }
                setTimeout(continueRunning, 0)
            }
        }

        continueRunning()
    }

    document.getElementById("step-btn").onclick = () => {
        if (finished) return
        document.getElementById("reset-btn").disabled = false

        if (origSrc == null) {
            const src = editor.getSrc()
            origSrc = src
            interpreter.replaceSrc(src)
            editor.disableEditing()
        }
        let result = interpreter.step()
        // Get IP location(s)
        const ipCount = interpreter.ipCount()
        let ipLocations = []
        for (let i = 0; i < ipCount; ++i) {
            ipLocations.push(interpreter.ipLocation(i))
        }
        editor.setSrc(interpreter.getSrcLines(), ipLocations)

        if (result != undefined) {
            finished = true
            document.getElementById("step-btn").disabled = true
            document.getElementById("run-btn").disabled = true
            document.getElementById("returncode-info").innerText = `Exited with status ${result}`
        }
    }

    document.getElementById("reset-btn").onclick = () => {
        if (origSrc != null) {
            editor.setSrc(origSrc)
            origSrc = null
        }
        interpreter.close()
        interpreter.init()
        finished = false
        stop = false
        document.getElementById("step-btn").disabled = false
        document.getElementById("run-btn").disabled = false
        document.getElementById("stop-btn").disabled = true
        document.getElementById("reset-btn").disabled = true
        editor.enableEditing()
        document.getElementById("returncode-info").innerText = ""
        document.getElementById("output").innerHTML = ""
        document.getElementById("warnings").innerHTML = ""
    }

    document.getElementById("stop-btn").onclick = () => {
        stop = true
    }
}

window.addEventListener("load", initialize)

