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
    body: "opengl window context"
}).then((res) => console.log(res));