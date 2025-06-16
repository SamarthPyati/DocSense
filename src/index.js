console.log("Querying /api/search")

fetch("/api/search", {
    method: 'POST', 
    mode: 'cors', 
    cache: 'no-cache', 
    credentials: 'same-origin', 
    headers: {
        'Content-Type': 'text/plain'
    }, 
    redirect: 'follow', 
    referrerPolicy: 'no-referrer', 
    body: "How To opengl or open-gl frame buffer context window?"
}).then((res) => console.log(res)); 