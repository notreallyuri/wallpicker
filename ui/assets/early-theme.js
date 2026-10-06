// Runs before the first frame: dark or light, the color theme and the last
// wallpaper palette, as the app sets them once it starts. Without it the page
// is drawn in the light default for a moment, then switches: a white flash
// when the window opens. Its own file because the CSP allows no inline script.
// Keys: nocomponents' ThemeProvider ("ui-theme", "ui-color-theme") and
// palette.rs ("wallpicker.ui.palette_css").
(function () {
  try {
    var store = window.localStorage;
    var html = document.documentElement;
    var mode = store.getItem("ui-theme") || "system";
    var dark = mode === "dark" || (mode !== "light" && window.matchMedia("(prefers-color-scheme: dark)").matches);
    if (dark) html.classList.add("dark");
    var color = store.getItem("ui-color-theme");
    if (color && /^[a-z0-9-]+$/.test(color)) html.setAttribute("data-theme", color);
    var css = store.getItem("wallpicker.ui.palette_css");
    if (css) {
      var style = document.createElement("style");
      style.id = "wallpaper-palette";
      style.textContent = css;
      document.head.appendChild(style);
    }
  } catch (e) {
    // No storage: the app's defaults, a moment later.
  }
})();
