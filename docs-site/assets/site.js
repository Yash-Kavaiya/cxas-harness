(() => {
  const NAV = [
    {
      title: "Start",
      items: [
        ["index.html", "Overview"],
        ["getting-started.html", "Getting started"],
        ["examples.html", "Examples"],
      ],
    },
    {
      title: "Product",
      items: [
        ["architecture.html", "Architecture"],
        ["cli.html", "CLI reference"],
        ["catalog.html", "Local catalog"],
        ["crates.html", "Crate SDK"],
      ],
    },
    {
      title: "Engines",
      items: [
        ["lint.html", "Lint engine"],
        ["evals.html", "Evals and audio"],
        ["migration.html", "Migration"],
      ],
    },
    {
      title: "Project",
      items: [
        ["issues.html", "Issue coverage"],
        ["design.html", "Design specs"],
        ["limits.html", "Limits and honesty"],
        ["website.html", "This website"],
      ],
    },
  ];

  const page = (document.body.dataset.page || "index.html").replace(/^\.\//, "");
  const template = document.getElementById("page");
  const pageContent = template ? template.content.cloneNode(true) : null;
  const year = new Date().getFullYear();

  const navHtml = NAV.map((group) => {
    const links = group.items
      .map(([href, label]) => {
        const active = href === page ? " active" : "";
        return `<a class="${active.trim()}" href="${href}">${label}</a>`;
      })
      .join("");
    return `<nav class="nav-group"><h2>${group.title}</h2>${links}</nav>`;
  }).join("");

  document.body.innerHTML = `
    <a class="skip" href="#content">Skip to content</a>
    <header class="topbar">
      <button class="menu-btn" type="button" aria-label="Open navigation">Menu</button>
      <a class="brand" href="index.html">
        <img src="assets/favicon.svg" alt="">
        cxas-harness
        <span>docs</span>
      </a>
      <form class="top-search" role="search">
        <input type="search" id="q" placeholder="Filter pages…" autocomplete="off">
      </form>
      <div class="top-links">
        <a href="https://github.com/Yash-Kavaiya/cxas-harness">GitHub</a>
        <a href="https://github.com/Yash-Kavaiya/cxas-harness/blob/master/README.md">README</a>
      </div>
    </header>
    <div class="layout">
      <aside class="sidebar" id="sidebar">${navHtml}</aside>
      <main class="main">
        <article class="page" id="content"></article>
        <footer class="footer">
          Apache-2.0 · Independent rewrite of Google Cloud cxas-scrapi · not an official Google product · ${year}
        </footer>
      </main>
    </div>
  `;

  const content = document.getElementById("content");
  if (pageContent) content.appendChild(pageContent);

  const sidebar = document.getElementById("sidebar");
  const menuBtn = document.querySelector(".menu-btn");
  menuBtn.addEventListener("click", () => sidebar.classList.toggle("open"));
  document.addEventListener("click", (event) => {
    if (!sidebar.classList.contains("open")) return;
    if (sidebar.contains(event.target) || menuBtn.contains(event.target)) return;
    sidebar.classList.remove("open");
  });

  const search = document.getElementById("q");
  search.addEventListener("input", () => {
    const q = search.value.trim().toLowerCase();
    sidebar.querySelectorAll("a").forEach((a) => {
      a.style.display = !q || a.textContent.toLowerCase().includes(q) ? "" : "none";
    });
  });

  if (window.mermaid) {
    window.mermaid.initialize({
      startOnLoad: false,
      theme: "dark",
      securityLevel: "loose",
    });
    window.mermaid.run({ querySelector: ".mermaid" });
  }
})();
