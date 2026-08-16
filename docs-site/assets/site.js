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
        ["benchmark.html", "Benchmark and Gauntlet"],
        ["issues.html", "Issue coverage"],
        ["design.html", "Design specs"],
        ["limits.html", "Limits and honesty"],
        ["website.html", "This website"],
      ],
    },
  ];

  const page = (document.body.dataset.page || "index.html").replace(/^\.\//, "");
  const article = document.getElementById("content");
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

  const skip = document.createElement("a");
  skip.className = "skip";
  skip.href = "#content";
  skip.textContent = "Skip to content";

  const header = document.createElement("header");
  header.className = "topbar";
  header.innerHTML = `
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
      </div>`;

  const layout = document.createElement("div");
  layout.className = "layout";
  const sidebar = document.createElement("aside");
  sidebar.className = "sidebar";
  sidebar.id = "sidebar";
  sidebar.innerHTML = navHtml;
  const main = document.createElement("main");
  main.className = "main";
  const footer = document.createElement("footer");
  footer.className = "footer";
  footer.textContent = `Apache-2.0 · Independent rewrite of Google Cloud cxas-scrapi · not an official Google product · ${year}`;

  if (article) {
    article.classList.add("page");
    main.appendChild(article);
  }
  main.appendChild(footer);
  layout.appendChild(sidebar);
  layout.appendChild(main);

  document.body.prepend(skip);
  document.body.insertBefore(header, skip.nextSibling);
  document.body.insertBefore(layout, header.nextSibling);

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
