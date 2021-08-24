export function writeOutput(s)
{
    let output_elem = document.getElementById("output")

    output_elem.innerHTML = (output_elem.innerHTML
        + s.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;"))
}

export function writeWarning(msg)
{
    let warning_elem = document.getElementById("warnings")

    warning_elem.innerHTML = (warning_elem.innerHTML
        + msg.replaceAll("<", "&lt;").replaceAll(">", "&gt;")
        + "<br>")
}

export function getEnvVars()
{
    return {
        "USER_AGENT": navigator.userAgent,
        "HREF": window.location.href,
        "PATH": window.location.pathname,
        "HOST": window.location.host,
        "QUERY": window.location.search,
        "HASH": window.location.hash,
        "CONTENT_TYPE": document.contentType,
        "CHARSET": document.charset
    }
}