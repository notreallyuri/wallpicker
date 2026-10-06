// Run in the window by an automation build (WALLPICKER_AUTOMATE), a few
// seconds after it was built: waits until thumbnails stop arriving, then
// reports the page's timing marks to the app (`open_timing`), which prints
// them with when the window was asked for and built.
(async () => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const thumbs = () => performance.getEntriesByName('wp:thumbnail').length;
  // Settled: no new thumbnail for 1.5 s (or 15 s in all).
  let last = -1, still = 0;
  for (let i = 0; i < 150 && still < 15; i++) {
    const n = thumbs();
    still = n === last ? still + 1 : 0;
    last = n;
    await sleep(100);
  }
  const marks = performance.getEntriesByType('mark').filter((m) => m.name.startsWith('wp:'));
  const named = {};
  for (const m of marks) if (m.name !== 'wp:thumbnail') named[m.name.slice(3)] = Math.round(m.startTime);
  const t = marks.filter((m) => m.name === 'wp:thumbnail').map((m) => m.startTime);
  const nav = performance.getEntriesByType('navigation')[0];
  const report = {
    origin: Math.round(performance.timeOrigin),
    dom_ready: nav ? Math.round(nav.domContentLoadedEventEnd) : null,
    loaded: nav ? Math.round(nav.loadEventEnd) : null,
    paint: Object.fromEntries(performance.getEntriesByType('paint').map((p) => [p.name, Math.round(p.startTime)])),
    marks: named,
    thumbnails: t.length,
    thumbnails_done: t.length ? Math.round(Math.max(...t)) : null,
    resources: performance.getEntriesByType('resource')
      .filter((r) => /\.(wasm|js|css|ttf)(\?|$)/.test(r.name))
      .map((r) => ({ name: r.name.split('/').pop().slice(0, 40), start: Math.round(r.startTime), end: Math.round(r.responseEnd) })),
  };
  window.__TAURI__.core.invoke('open_timing', { report: JSON.stringify(report) });
})();
