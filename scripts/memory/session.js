// Replays a browsing session: Online, 4 pages, 4 details, Konachan, Local,
// scrolling. VARIANT (prepended by run.sh) switches parts off to see what
// costs memory. Run in the window by a `--features automation` build.
(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const log = (m) => { (window.__steps = window.__steps || []).push([Math.round(performance.now() / 1000), m]); };
  if (VARIANT === 'nofx') {
    const s = document.createElement('style');
    s.textContent = '*, *::before, *::after { backdrop-filter: none !important; -webkit-backdrop-filter: none !important; filter: none !important; transition: none !important; animation: none !important; }';
    document.head.appendChild(s);
  }
  if (VARIANT === 'noimg') {
    const s = document.createElement('style');
    s.textContent = 'img { display: none !important; }';
    document.head.appendChild(s);
  }
  const byText = (sel, text) => [...document.querySelectorAll(sel)].find((e) => e.textContent.trim() === text);
  const scrollers = () => [...document.querySelectorAll('*')].filter((e) => { const st = getComputedStyle(e); return /(auto|scroll)/.test(st.overflowY) && e.scrollHeight > e.clientHeight + 50 && e.offsetParent; });
  const scrollDown = async (n) => { for (let i = 0; i < n; i++) { for (const el of scrollers()) el.scrollBy(0, 700); await sleep(400); } };
  await sleep(3000);
  byText('[role=tab]', 'Online')?.click(); log('online');
  await sleep(6000);
  for (let p = 0; p < 4; p++) {
    await scrollDown(4);
    if (p < 3) {
      const next = [...document.querySelectorAll('a, button')].find((e) => /next/i.test(e.getAttribute('aria-label') || '') || /^next/i.test(e.textContent.trim()));
      next?.click(); log('next page ' + !!next);
      await sleep(5000);
    }
  }
  for (let d = 0; d < 4; d++) {
    const buttons = [...document.querySelectorAll('button')].filter((b) => b.textContent.trim() === 'Details');
    buttons[d * 3]?.click(); log('details ' + buttons.length);
    await sleep(5000);
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    document.activeElement?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await sleep(1500);
  }
  byText('[role=tab]', 'Konachan')?.click(); log('konachan');
  await sleep(6000);
  await scrollDown(6);
  byText('[role=tab]', 'Local')?.click(); log('local');
  await sleep(3000);
  await scrollDown(12);
  log('done');
  document.title = 'done:' + JSON.stringify(window.__steps);
})();
