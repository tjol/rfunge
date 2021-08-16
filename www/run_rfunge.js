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

    document.getElementById("run-btn").onclick = () => {
        if (finished) return
        interpreter.replaceSrc(editor.getSrc())
        let result = interpreter.run()
        finished = true
        document.getElementById("step-btn").disabled = true
        document.getElementById("run-btn").disabled = true
        document.getElementById("returncode-info").innerText = `Exited with status ${result}`
    }

    document.getElementById("step-btn").onclick = () => {
        if (finished) return

        interpreter.replaceSrc(editor.getSrc())
        let result = interpreter.step()
        // Get IP location(s)
        const ipCount = interpreter.ipCount()
        let ipLocations = []
        for (let i = 0; i < ipCount; ++i) {
            ipLocations.push(interpreter.ipLocation(i))
        }
        editor.setSrc(interpreter.getSrc(), ipLocations)

        if (result != undefined) {
            finished = true
            document.getElementById("step-btn").disabled = true
            document.getElementById("run-btn").disabled = true
            document.getElementById("returncode-info").innerText = `Exited with status ${result}`
        }
    }

    document.getElementById("reset-btn").onclick = () => {
    }
}

window.addEventListener("load", initialize)

