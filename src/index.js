// Fetch + display index stats in the footer
async function loadStats() {
  try {
    const res = await fetch("/api/stats");
    if (!res.ok) return;
    const { doc_count, unique_term_count } = await res.json();
    document.getElementById("stat-docs").textContent = doc_count.toLocaleString();
    document.getElementById("stat-terms").textContent = unique_term_count.toLocaleString();
  } catch (_) { /* stats are non-critical */ }
}

// Derive a file extension from an absolute path string
function extOf(path) {
  const dot = path.lastIndexOf(".");
  return dot !== -1 ? path.slice(dot + 1).toLowerCase() : "";
}

// Map extension → badge CSS class
function extClass(ext) {
  if (["xml", "xhtml"].includes(ext)) return "ext-xml";
  if (ext === "pdf") return "ext-pdf";
  if (["txt", "md"].includes(ext)) return "ext-txt";
  return "ext-default";
}

// "path/to/file.pdf" → filename "file.pdf" + dir "path/to"
function splitPath(fullPath) {
  const sep = fullPath.includes("/") ? "/" : "\\";
  const idx = fullPath.lastIndexOf(sep);
  if (idx === -1) return { name: fullPath, dir: "" };
  return { name: fullPath.slice(idx + 1), dir: fullPath.slice(0, idx) };
}

// Render the results list into #results
function renderResults(data) {
  const container = document.getElementById("results");
  container.innerHTML = "";

  if (data.length === 0) {
    container.innerHTML = `
      <div class="state-msg">
        <svg width="32" height="32" viewBox="0 0 24 24" fill="none">
          <circle cx="11" cy="11" r="7" stroke="currentColor" stroke-width="1.5"/>
          <path d="M16.5 16.5L21 21" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
          <path d="M8 11h6" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        No results found
      </div>`;
    return;
  }

  const header = document.createElement("div");
  header.className = "results-header";
  header.textContent = `${data.length} result${data.length !== 1 ? "s" : ""}`;
  container.appendChild(header);

  const list = document.createElement("div");
  list.className = "result-list";

  for (const [path, rank] of data) {
    const ext = extOf(path);
    const { name, dir } = splitPath(path);

    const a = document.createElement("a");
    a.className = "result-item";
    a.href = "/file?path=" + encodeURIComponent(path);
    a.target = "_blank";
    a.rel = "noopener noreferrer";
    a.setAttribute("aria-label", name);

    a.innerHTML = `
      <div class="ext-badge ${extClass(ext)}">${ext || "?"}</div>
      <div class="result-body">
        <div class="result-filename">${escHtml(name)}</div>
        ${dir ? `<div class="result-path">${escHtml(dir)}</div>` : ""}
      </div>
      <div class="result-rank">${rank.toFixed(3)}</div>`;

    list.appendChild(a);
  }

  container.appendChild(list);
}

function escHtml(str) {
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

function renderError(msg) {
  document.getElementById("results").innerHTML =
    `<div class="state-msg error">
      <svg width="28" height="28" viewBox="0 0 24 24" fill="none">
        <circle cx="12" cy="12" r="9" stroke="currentColor" stroke-width="1.5"/>
        <path d="M12 8v4" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
        <circle cx="12" cy="16" r="0.75" fill="currentColor"/>
      </svg>
      ${escHtml(msg)}
    </div>`;
}

// Core search function
const spinner = document.getElementById("spinner");
const hint = document.getElementById("hint");

async function search(prompt) {
  if (!prompt.trim()) {
    document.getElementById("results").innerHTML = "";
    return;
  }

  hint.classList.add("hidden");
  spinner.classList.add("active");

  try {
    const res = await fetch("/api/search", {
      method: "POST",
      mode: "cors",
      cache: "no-cache",
      credentials: "same-origin",
      headers: { "Content-Type": "text/plain" },
      redirect: "follow",
      referrerPolicy: "no-referrer",
      body: prompt,
    });

    if (!res.ok) {
      renderError(`Server error ${res.status}`);
      return;
    }

    const data = await res.json();
    renderResults(data);
  } catch (err) {
    renderError("Could not reach the search server.");
  } finally {
    spinner.classList.remove("active");
    hint.classList.remove("hidden");
  }
}

// Input wiring
const queryEl = document.getElementById("query");

queryEl.addEventListener("keydown", (e) => {
  if (e.key === "Enter") search(queryEl.value);
});

// Hide the ↵ hint while typing, show it again when empty
queryEl.addEventListener("input", () => {
  hint.classList.toggle("hidden", queryEl.value.length > 0);
});

// Load footer stats on page load
loadStats();