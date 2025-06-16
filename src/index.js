console.log("Querying /api/search")

async function search(prompt) {
    let results = document.getElementById("results");
    results.innerHTML = "";
    const response = await fetch("/api/search", {
        method: 'POST', 
        mode: 'cors', 
        cache: 'no-cache', 
        credentials: 'same-origin', 
        headers: {
            'Content-Type': 'text/plain'
        }, 
        redirect: 'follow', 
        referrerPolicy: 'no-referrer', 
        body: prompt
    });

    for ([path, rank] of await response.json()) {
        if (response.json === null) {
            let item = document.createElement("span");
            item.appendChild(document.createTextNode("No such token found"));
            item.appendChild(document.createElement("br"));    
        }
        
        let item = document.createElement("span");
        item.appendChild(document.createTextNode(path));
        item.appendChild(document.createElement("br"));
        results.appendChild(item);
    }
}

let query = document.getElementById("query");

query.addEventListener("keypress", (e) => {
        if (e.key == "Enter") {
            search(query.value);
        }
})