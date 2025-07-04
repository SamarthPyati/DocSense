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

    const data = await response.json();
    if (data.length === 0) {
        let item = document.createElement("span");
        item.textContent = "No results found.";
        results.appendChild(item);
    } else {
        for (let [path, rank] of data) {
            let link = document.createElement("a");
            link.href = path;
            link.target = "_blank"; // open in new tab
            link.textContent = path;
            results.appendChild(link);
            results.appendChild(document.createElement("br"));
        }
    }
}

let query = document.getElementById("query");
let currentSearch = Promise.resolve();
query.addEventListener("keypress", (e) => {
        if (e.key == "Enter") {
            currentSearch.then(() => {
                search(query.value);
            });
        }
})