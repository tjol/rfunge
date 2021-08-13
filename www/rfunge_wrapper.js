import init_rfunge_wasm, * as rfunge from "./wasm_pkg/rfunge.js"


function run_wasm_program() {
    // empty output
    document.getElementById("output").innerHTML = ""

    // create interpreter
    let interpreter = rfunge.new_interpreter()
    let src = document.getElementById("src-input").value

    rfunge.load_src(interpreter, src)
    rfunge.run(interpreter)
    rfunge.free_interpreter(interpreter)
}

window.addEventListener("load", async (e) => {
    await init_rfunge_wasm()
    document.getElementById("run-btn").onclick = run_wasm_program
})

