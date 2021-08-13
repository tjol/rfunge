export function write_rfunge_output(s)
{
    let output_elem = document.getElementById("output")

    output_elem.innerHTML = output_elem.innerHTML + s
}

export function write_rfunge_warning(msg)
{
    alert(msg)
}