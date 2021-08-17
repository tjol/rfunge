export function write_rfunge_output(s)
{
    let output_elem = document.getElementById("output")

    output_elem.innerHTML = (output_elem.innerHTML
        + s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;"))
}

export function write_rfunge_warning(msg)
{
    let warning_elem = document.getElementById("warnings")

    warning_elem.innerHTML = (warning_elem.innerHTML
        + msg.replaceAll("<", "&lt;").replaceAll(">", "&gt;")
        + "<br>")
}