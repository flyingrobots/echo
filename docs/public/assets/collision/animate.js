// SPDX-License-Identifier: Apache-2.0
// © James Ross Ω FLYING•ROBOTS <https://github.com/flyingrobots>
(function(){
  const els = Array.from(document.querySelectorAll('.fade-seed'));
  if (!('IntersectionObserver' in window)) {
    els.forEach(el => el.classList.add('fade-in'));
    return;
  }
  const io = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        entry.target.classList.add('fade-in');
        io.unobserve(entry.target);
      }
    });
  }, { rootMargin: '0px 0px -10% 0px', threshold: 0.1 });
  els.forEach(el => io.observe(el));
})();

// Carousel pager for each rule's step-grid
(function(){
  function buildPager(rule) {
    const grid = rule.querySelector('.step-grid');
    if (!grid) return;
    const slides = Array.from(grid.querySelectorAll('figure'));
    if (slides.length < 2) return;

    // Build overlay captions with step counts from figcaptions
    slides.forEach((fig, i) => {
      const cap = fig.querySelector('figcaption');
      const text = cap ? cap.textContent.trim() : '';
      const ov = document.createElement('div');
      ov.className = 'overlay';
      ov.innerHTML = `<strong>Step ${i + 1} of ${slides.length}</strong><div class="ov-t">${text}</div>`;
      fig.appendChild(ov);
      fig.classList.add('has-overlay');
    });

    const nav = document.createElement('div');
    nav.className = 'pager';
    const prev = document.createElement('button');
    prev.className = 'btn'; prev.textContent = '◀ Prev';
    const next = document.createElement('button');
    next.className = 'btn'; next.textContent = 'Next ▶';
    const toggle = document.createElement('button');
    toggle.className = 'btn'; toggle.textContent = 'Show all';
    const world = document.createElement('button');
    world.className = 'btn'; world.textContent = 'World view: On';
    nav.append(prev, next, toggle, world);
    grid.after(nav);

    let mode = 'all'; // default to all frames visible
    let idx = 0;

    // Helpers to find neighbor rule blocks
    const prevRule = (el) => { let r = el.previousElementSibling; while (r && !r.classList.contains('rule')) r = r.previousElementSibling; return r; };
    const nextRule = (el) => { let r = el.nextElementSibling; while (r && !r.classList.contains('rule')) r = r.nextElementSibling; return r; };

    function render() {
      if (mode === 'all') {
        slides.forEach(el => el.classList.remove('hidden'));
        toggle.textContent = 'Carousel mode';
      } else {
        slides.forEach((el, i) => {
          el.classList.toggle('hidden', i !== idx);
          // ensure visible slide is faded in
          if (!el.classList.contains('fade-in')) el.classList.add('fade-in');
        });
        toggle.textContent = 'Show all';
      }
      // Enable/disable edges. If at first slide and no previous rule, disable Prev.
      // If at last slide and no next rule, disable Next.
      if (mode === 'all') {
        // Keep navigation enabled in 'all' mode so users/tests can enter carousel via Prev/Next.
        prev.disabled = false; next.disabled = false;
      } else {
        const atFirst = idx === 0;
        const atLast = idx === slides.length - 1;
        prev.disabled = atFirst && !prevRule(rule);
        next.disabled = atLast && !nextRule(rule);
      }
    }

    prev.addEventListener('click', () => {
      // If in all mode, enter carousel at first slide
      if (mode === 'all') { mode = 'one'; idx = 0; render(); return; }
      if (idx > 0) { idx -= 1; render(); return; }
      // At first slide: navigate to previous rule, show its first slide
      const pr = prevRule(rule);
      if (pr && pr._pager) {
        pr._pager.setIndex(0);
        pr.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }
      render();
    });
    next.addEventListener('click', () => {
      if (mode === 'all') { mode = 'one'; idx = 0; render(); return; }
      if (idx < slides.length - 1) { idx += 1; render(); return; }
      // At last slide: navigate to next rule, show its first slide
      const nr = nextRule(rule);
      if (nr && nr._pager) {
        nr._pager.setIndex(0);
        nr.scrollIntoView({ behavior: 'smooth', block: 'start' });
      }
      render();
    });
    toggle.addEventListener('click', () => {
      mode = (mode === 'all') ? 'one' : 'all';
      render();
    });

    // Picture-in-picture container with tabs (World / Graph)
    slides.forEach((fig) => {
      const srcWorld = fig.getAttribute('data-pip');
      const srcGraph = fig.getAttribute('data-graph');
      if (!srcWorld && !srcGraph) return;
      const wrap = document.createElement('div');
      wrap.className = 'pip';
      const tabs = document.createElement('div');
      tabs.className = 'pip-tabs';
      const tabWorld = document.createElement('div'); tabWorld.className = 'tab active'; tabWorld.textContent = 'World';
      const tabGraph = document.createElement('div'); tabGraph.className = 'tab'; tabGraph.textContent = 'Graph';
      tabs.append(tabWorld, tabGraph);
      const imgWorld = document.createElement('img'); imgWorld.alt = 'World view'; if (srcWorld) imgWorld.src = srcWorld; else imgWorld.style.display='none';
      const imgGraph = document.createElement('img'); imgGraph.alt = 'Graph view'; if (srcGraph) imgGraph.src = srcGraph; else imgGraph.style.display='none'; imgGraph.classList.add('hidden');
      wrap.append(tabs, imgWorld, imgGraph);
      fig.appendChild(wrap);
      function show(which){
        if (which==='world') { tabWorld.classList.add('active'); tabGraph.classList.remove('active'); imgWorld.classList.remove('hidden'); imgGraph.classList.add('hidden'); }
        else { tabGraph.classList.add('active'); tabWorld.classList.remove('active'); imgGraph.classList.remove('hidden'); imgWorld.classList.add('hidden'); }
      }
      tabWorld.addEventListener('click', ()=>show('world'));
      tabGraph.addEventListener('click', ()=>show('graph'));
    });

    let worldOn = true;
    world.addEventListener('click', () => {
      worldOn = !worldOn;
      world.textContent = worldOn ? 'World view: On' : 'World view: Off';
      slides.forEach(fig => {
        const pip = fig.querySelector('.pip');
        if (pip) pip.classList.toggle('hidden', !worldOn);
      });
    });

    // Expose simple API for cross-rule navigation
    rule._pager = {
      setIndex: (i) => { mode = 'one'; idx = Math.max(0, Math.min(slides.length - 1, i)); render(); },
      setMode: (m) => { mode = m; render(); },
    };

    render();
  }

  document.querySelectorAll('.rule').forEach(buildPager);
})();
