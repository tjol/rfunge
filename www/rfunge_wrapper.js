import init_rfunge_wasm, * as rfunge from "./wasm_pkg/rfunge.js"


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

window.addEventListener("load", async (e) => {
    await init_rfunge_wasm()
    document.getElementById("run-btn").onclick = run_wasm_program
})

