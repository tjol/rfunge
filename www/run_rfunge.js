import init_rfunge_wasm, * as rfunge from "./wasm_pkg/rfunge.js"
import * as fse from "./fungespace_edit.js";

function run_wasm_program() {
    // empty output
    document.getElementById("output").innerHTML = ""

    // create interpreter
    let interpreter = new rfunge.BefungeInterpreter()
    let src = document.getElementById("src-input").value

    interpreter.init()
    interpreter.load_src(src)
    interpreter.run()
    interpreter.close()
}

async function initialize() {
    await init_rfunge_wasm()
    let editor = new fse.FungeSpaceEdit("fungespace")
    let interpreter = new rfunge.BefungeInterpreter()
    interpreter.init()
    let finished = false
    let origSrc = null
    let lastSrc = null
    let stop = false

    document.getElementById("run-btn").onclick = () => {
        if (finished) return
        const src = editor.getSrc()
        origSrc = src
        interpreter.replaceSrc(src)
        document.getElementById("step-btn").disabled = true
        document.getElementById("run-btn").disabled = true
        document.getElementById("reset-btn").disabled = true
        document.getElementById("stop-btn").disabled = false
        
        function continueRunning() {
            if (stop) {
                finished = true
                document.getElementById("returncode-info").innerText = "Aborted by user"
                document.getElementById("reset-btn").disabled = false
                document.getElementById("stop-btn").disabled = true
                return
            }
            // Return control to the main event loop after every step
            // so we don't hang the browser
            let result = interpreter.step()
            if (result != null) {
                finished = true
                document.getElementById("returncode-info").innerText = `Exited with status ${result}`
                document.getElementById("reset-btn").disabled = false
                document.getElementById("stop-btn").disabled = true
            } else {
                setTimeout(continueRunning, 0)
            }
        }

        continueRunning()
    }

    document.getElementById("step-btn").onclick = () => {
        if (finished) return

        const src = editor.getSrc()
        if (origSrc == null) {
            origSrc = src
            lastSrc = null
        } else if (lastSrc != null && src.trim() !== lastSrc.trim()) {
            // user has edited the source
            origSrc = src
        }
        interpreter.replaceSrc(src)
        let result = interpreter.step()
        // Get IP location(s)
        const ipCount = interpreter.ipCount()
        let ipLocations = []
        for (let i = 0; i < ipCount; ++i) {
            ipLocations.push(interpreter.ipLocation(i))
        }
        const newSrc = interpreter.getSrc()
        editor.setSrc(newSrc, ipLocations)
        lastSrc = newSrc

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
            lastSrc = null
        }
        interpreter.close()
        interpreter.init()
        finished = false
        stop = false
        document.getElementById("step-btn").disabled = false
        document.getElementById("run-btn").disabled = false
        document.getElementById("stop-btn").disabled = true
        document.getElementById("reset-btn").disabled = false
        document.getElementById("returncode-info").innerText = ""
        document.getElementById("output").innerText = ""
    }

    document.getElementById("stop-btn").onclick = () => {
        stop = true
    }
}

window.addEventListener("load", initialize)

