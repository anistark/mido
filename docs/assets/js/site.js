const $ = (s, r = document) => r.querySelector(s);
const $$ = (s, r = document) => [...r.querySelectorAll(s)];

const outlineLinks = $$("[data-outline]");
if (outlineLinks.length && "IntersectionObserver" in window) {
  const byId = new Map(outlineLinks.map((a) => [a.dataset.outline, a]));
  const heads = [...byId.keys()].map((id) => document.getElementById(id)).filter(Boolean);
  let current = null;
  const setCurrent = (id) => {
    if (id === current) return;
    current = id;
    outlineLinks.forEach((a) => a.toggleAttribute("aria-current", a.dataset.outline === id));
  };
  const pick = () => {
    const line = window.innerHeight * 0.25;
    let best = heads[0];
    for (const h of heads) {
      if (h.getBoundingClientRect().top <= line) best = h;
    }
    if (best) setCurrent(best.id);
  };
  const io = new IntersectionObserver(pick, { rootMargin: "-20% 0px -60% 0px", threshold: [0, 1] });
  heads.forEach((h) => io.observe(h));
  window.addEventListener("scroll", pick, { passive: true });
  pick();
}

const toggle = $(".nav-toggle");
const files = $("#files");
if (toggle && files) {
  toggle.hidden = false;
  toggle.addEventListener("click", () => {
    const open = document.body.classList.toggle("nav-open");
    toggle.setAttribute("aria-expanded", String(open));
  });
  files.addEventListener("click", (e) => {
    if (e.target.closest("a")) document.body.classList.remove("nav-open");
  });
}

$$("pre[class*='language-']").forEach((pre) => {
  const btn = document.createElement("button");
  btn.type = "button";
  btn.className = "copy";
  btn.textContent = "copy";
  btn.addEventListener("click", async () => {
    try {
      await navigator.clipboard.writeText(pre.innerText);
      btn.textContent = "copied";
    } catch {
      btn.textContent = "select and copy";
    }
    setTimeout(() => (btn.textContent = "copy"), 1600);
  });
  pre.appendChild(btn);
});

const term = $("#term");
const replay = $(".term-replay");
if (term && replay && !matchMedia("(prefers-reduced-motion: reduce)").matches) {
  replay.hidden = false;
  replay.addEventListener("click", () => {
    term.classList.remove("play");
    void term.offsetWidth;
    term.classList.add("play");
  });
  $$(".term-doc .row").forEach((row, i) => row.style.setProperty("--i", i));
  term.classList.add("play");
}
