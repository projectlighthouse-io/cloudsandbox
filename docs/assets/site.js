{
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => [...r.querySelectorAll(s)];
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
  for (const k of $$("[data-modkey]")) k.textContent = isMac ? "⌘ K" : "Ctrl K";

  /* ---------- enter reveal: once, staggered within a row ---------- */
  const revealObserver = new IntersectionObserver((entries) => {
    for (const en of entries) {
      if (!en.isIntersecting) continue;
      en.target.classList.add("in");
      revealObserver.unobserve(en.target);
    }
  }, { rootMargin: "0px 0px -8% 0px" });
  for (const el of $$(".reveal")) {
    const sibs = el.parentElement ? $$(":scope > .reveal", el.parentElement) : [];
    el.style.setProperty("--d", Math.max(0, sibs.indexOf(el)) * 60 + "ms");
    reduce ? el.classList.add("in") : revealObserver.observe(el);
  }

  /* ---------- live mocks: one behaviour each, only while in view ---------- */
  function makeLive(root) {
    const kind = root.dataset.live;
    const step = +root.dataset.step || 1000;
    let timer = 0, i = 0, running = false, paused = false;

    const seqItems = $$("[data-s]", root);
    const cycleItems = $$("[data-c]", root);
    const stepItems = $$("[data-st]", root);
    const fillItems = $$("[data-f]", root);

    const final = () => {
      seqItems.forEach((e) => e.classList.add("on"));
      stepItems.forEach((e) => { e.classList.add("done"); e.classList.remove("cur"); });
      if (cycleItems.length) { const f = cycleItems.find((e) => e.hasAttribute("data-final")) || cycleItems[0]; cycleItems.forEach((e) => e.classList.toggle("sel", e === f)); }
      fillItems.forEach((e) => { e.style.transition = "none"; e.classList.add("go"); });
    };
    const reset = () => {
      seqItems.forEach((e) => e.classList.remove("on", "now"));
      stepItems.forEach((e) => e.classList.remove("done", "cur"));
      fillItems.forEach((e) => { e.style.transition = "none"; e.classList.remove("go"); void e.offsetWidth; e.style.transition = ""; });
      i = 0;
    };
    const tick = () => {
      if (paused) { timer = setTimeout(tick, 400); return; }
      if (kind === "seq") {
        if (i < seqItems.length) {
          seqItems.forEach((e) => e.classList.remove("now"));
          seqItems[i].classList.add("on", "now");
          i++;
          timer = setTimeout(tick, step);
        } else {
          seqItems.forEach((e) => e.classList.remove("now"));
          timer = setTimeout(() => { reset(); timer = setTimeout(tick, 500); }, 3200);
        }
      } else if (kind === "cycle") {
        cycleItems.forEach((e, k) => e.classList.toggle("sel", k === i % cycleItems.length));
        i++;
        timer = setTimeout(tick, i % cycleItems.length === 0 ? step + 1400 : step);
      } else if (kind === "steps") {
        if (i <= stepItems.length) {
          stepItems.forEach((e, k) => { e.classList.toggle("done", k < i); e.classList.toggle("cur", k === i); });
          i++;
          timer = setTimeout(tick, step);
        } else {
          timer = setTimeout(() => { reset(); timer = setTimeout(tick, 400); }, 2600);
        }
      } else if (kind === "fill") {
        fillItems.forEach((e) => { e.style.setProperty("--fill", step - 400 + "ms"); e.classList.add("go"); });
        timer = setTimeout(() => { reset(); timer = setTimeout(tick, 600); }, step + 1600);
      }
    };
    if (reduce) { final(); return; }
    root.addEventListener("mouseenter", () => { paused = true; });
    root.addEventListener("mouseleave", () => { paused = false; });
    // hovering a row in a cycle mock takes the selection
    if (kind === "cycle") {
      cycleItems.forEach((e) => e.addEventListener("mouseenter", () => cycleItems.forEach((x) => x.classList.toggle("sel", x === e))));
    }
    const viewObserver = new IntersectionObserver(([en]) => {
      if (en.isIntersecting && !running) { running = true; reset(); tick(); }
      else if (!en.isIntersecting && running) { running = false; clearTimeout(timer); }
    }, { threshold: 0.35 });
    viewObserver.observe(root);
  }
  $$("[data-live]").forEach(makeLive);

  /* ---------- compose: a Cargo.toml that follows the ticks ---------- */
  const picks = $(".picks");
  if (picks) {
    const out = $("#cargo"), sum = $("#cargoSum");
    const features = { session: '["postgres"]', queue: '["postgres"]', db: '["postgres"]', inertia: '["validate"]', socialite: '["github"]' };
    let prev = new Set();
    const render = () => {
      const on = $$("input:checked", picks).map((i) => i.value);
      const width = Math.max(8, ...on.map((n) => n.length + 1));
      const lines = on.map((n) => {
        const pad = n.padEnd(width);
        const body = features[n]
          ? `{ version = <span class="ts">"0.1"</span>, features = <span class="ts">${features[n]}</span> }`
          : `<span class="ts">"0.1"</span>`;
        return `<span class="${prev.has(n) ? "" : "fresh"}">${esc(pad)}= ${body}</span>`;
      });
      out.innerHTML = '<span class="tk">[dependencies]</span>\n' + (lines.length ? lines.join("\n") : '<span class="tc"># tick a component to add it</span>');
      sum.textContent = on.length
        ? `${on.length} of 16 components · nothing else is pulled in`
        : "0 of 16 components";
      prev = new Set(on);
    };
    picks.addEventListener("change", render);
    render();
    prev = new Set($$("input:checked", picks).map((i) => i.value));
    out.querySelectorAll(".fresh").forEach((e) => e.classList.remove("fresh"));
  }

  /* ---------- copy buttons ---------- */
  const copy = async (btn, text) => {
    const label = btn.querySelector("span") || btn;
    const was = label.textContent;
    try { await navigator.clipboard.writeText(text); label.textContent = "Copied"; }
    catch (e) { label.textContent = "Select to copy"; }
    btn.classList.add("done");
    setTimeout(() => { label.textContent = was; btn.classList.remove("done"); }, 1200);
  };
  for (const b of $$("[data-copy-target]")) b.addEventListener("click", () => copy(b, $(b.dataset.copyTarget).innerText));
  for (const b of $$(".prose .copy")) b.addEventListener("click", () => copy(b, b.parentElement.querySelector("pre").innerText));
  const pageBtn = $(".copy-page");
  if (pageBtn) pageBtn.addEventListener("click", () => copy(pageBtn, $("#pageMd").textContent.replace(/<\\\//g, "</")));

  /* ---------- demo tabs: auto-advance, stop on hover or focus ---------- */
  for (const root of $$("[data-tabs]")) {
    const tabs = $$("[role=tab]", root);
    const panels = tabs.map((t) => document.getElementById(t.getAttribute("aria-controls")));
    let cur = 0, t0 = performance.now(), stopped = reduce, raf = 0, visible = false;
    const DURATION = 6000;
    const show = (k, focus) => {
      cur = k;
      tabs.forEach((t, j) => { t.setAttribute("aria-selected", String(j === k)); t.tabIndex = j === k ? 0 : -1; t.style.setProperty("--p", "0%"); });
      panels.forEach((p, j) => { p.hidden = j !== k; });
      if (focus) tabs[k].focus();
      t0 = performance.now();
    };
    const loop = (now) => {
      if (!stopped && visible) {
        const p = Math.min(1, (now - t0) / DURATION);
        tabs[cur].style.setProperty("--p", p * 100 + "%");
        if (p >= 1) show((cur + 1) % tabs.length);
      } else {
        t0 = now - parseFloat(tabs[cur].style.getPropertyValue("--p") || "0") / 100 * DURATION;
      }
      raf = requestAnimationFrame(loop);
    };
    tabs.forEach((t, k) => {
      t.addEventListener("click", () => { stopped = true; show(k); tabs[k].style.setProperty("--p", "100%"); });
      t.addEventListener("keydown", (e) => {
        if (e.key === "ArrowDown" || e.key === "ArrowRight") { e.preventDefault(); stopped = true; show((cur + 1) % tabs.length, true); }
        if (e.key === "ArrowUp" || e.key === "ArrowLeft") { e.preventDefault(); stopped = true; show((cur - 1 + tabs.length) % tabs.length, true); }
      });
    });
    root.addEventListener("mouseenter", () => { root.dataset.hover = "1"; stopped = true; });
    root.addEventListener("mouseleave", () => { delete root.dataset.hover; });
    root.addEventListener("focusin", () => { stopped = true; });
    show(0);
    new IntersectionObserver(([en]) => { visible = en.isIntersecting; }, { threshold: 0.3 }).observe(root);
    if (!reduce) raf = requestAnimationFrame(loop);
  }

  /* ---------- FAQ accordion ---------- */
  for (const b of $$(".qbtn")) {
    b.addEventListener("click", () => {
      const open = b.getAttribute("aria-expanded") !== "true";
      b.setAttribute("aria-expanded", String(open));
      b.closest(".qa").classList.toggle("open", open);
    });
  }

  /* ---------- docs: menu, on-this-page tracking, in-page anchors ---------- */
  const menu = $(".dmenu"), side = $("#dside");
  if (menu && side) {
    menu.addEventListener("click", () => {
      const open = side.classList.toggle("open");
      menu.setAttribute("aria-expanded", String(open));
    });
  }
  const tocLinks = $$(".dtoc a");
  if (tocLinks.length) {
    const byId = new Map(tocLinks.map((a) => [a.getAttribute("href").slice(1), a]));
    const tocObserver = new IntersectionObserver((entries) => {
      for (const en of entries) {
        if (!en.isIntersecting) continue;
        tocLinks.forEach((a) => a.classList.remove("on"));
        byId.get(en.target.id)?.classList.add("on");
      }
    }, { rootMargin: "-80px 0px -70% 0px" });
    for (const id of byId.keys()) { const h = document.getElementById(id); if (h) tocObserver.observe(h); }
  }

  /* ---------- command palette + shortcuts ---------- */
  const pal = $("#palette"), q = $("#palQ"), list = $("#palList"), keys = $("#keys");
  let hits = [], sel = 0, lastFocus = null;
  const rank = (term) => {
    const t = term.trim().toLowerCase();
    const docs = window.DOCS || [];
    if (!t) return docs.map((d) => ({ d, where: d.g, href: d.u }));
    const out = [];
    for (const d of docs) {
      let score = 0, where = d.g, href = d.u;
      const title = d.t.toLowerCase();
      if (title === t) score += 100; else if (title.startsWith(t)) score += 60; else if (title.includes(t)) score += 40;
      const h = d.h.find(([, x]) => x.toLowerCase().includes(t));
      if (h) { score += 20; where = d.g + " · " + h[1]; href = d.u + "#" + h[0]; }
      const i = d.x.toLowerCase().indexOf(t);
      if (i >= 0) { score += 5; if (!h && score < 40) where = "…" + d.x.slice(Math.max(0, i - 30), i + 60).trim() + "…"; }
      if (score) out.push({ d, where, href, score });
    }
    return out.sort((a, b) => b.score - a.score).slice(0, 12);
  };
  const draw = () => {
    hits = rank(q.value);
    sel = Math.min(sel, Math.max(0, hits.length - 1));
    list.innerHTML = hits.length
      ? hits.map((h, k) => `<li><a role="option" id="po${k}" href="${esc(h.href)}" aria-selected="${k === sel}"><b class="${h.d.m ? "mono" : ""}">${esc(h.d.t)}</b><small>${esc(h.where)}</small></a></li>`).join("")
      : '<li class="none">No matches. Try a component name such as route or queue.</li>';
    q.setAttribute("aria-activedescendant", hits.length ? "po" + sel : "");
    list.querySelector("[aria-selected=true]")?.scrollIntoView({ block: "nearest" });
  };
  const openPal = () => { lastFocus = document.activeElement; pal.hidden = false; q.value = ""; sel = 0; draw(); q.focus(); };
  const closeDialogs = () => {
    const wasOpen = !pal.hidden || !keys.hidden;
    pal.hidden = true; keys.hidden = true;
    if (wasOpen && lastFocus) lastFocus.focus();
  };
  for (const b of $$("[data-open-palette]")) b.addEventListener("click", openPal);
  pal.addEventListener("click", (e) => { if (e.target === pal) closeDialogs(); });
  keys.addEventListener("click", (e) => { if (e.target === keys || e.target.closest("[data-close]")) closeDialogs(); });
  q.addEventListener("input", () => { sel = 0; draw(); });
  q.addEventListener("keydown", (e) => {
    if (e.key === "ArrowDown") { e.preventDefault(); sel = Math.min(sel + 1, hits.length - 1); draw(); }
    else if (e.key === "ArrowUp") { e.preventDefault(); sel = Math.max(sel - 1, 0); draw(); }
    else if (e.key === "Enter" && hits[sel]) { location.href = hits[sel].href; closeDialogs(); }
  });
  document.addEventListener("keydown", (e) => {
    const typing = /input|textarea|select/i.test(document.activeElement?.tagName || "");
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") { e.preventDefault(); pal.hidden ? openPal() : closeDialogs(); }
    else if (e.key === "Escape") closeDialogs();
    else if (!typing && e.key === "/" && pal.hidden) { e.preventDefault(); openPal(); }
    else if (!typing && e.key === "?" && keys.hidden) { e.preventDefault(); lastFocus = document.activeElement; keys.hidden = false; keys.querySelector("button").focus(); }
  });
}
