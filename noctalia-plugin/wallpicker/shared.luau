--!nonstrict
-- What the panel's parts share: the state (`app.state`, S in each part),
-- the layout constants and the small helpers. Every part gets the same `app`
-- table and hangs its functions off it, so parts can call each other in any
-- order.

local Command = require("./command.luau")
local Theme = require("./theme.luau")
local Library = require("./library.luau")

local M = {}

function M.new(ui: any, noctalia: any, panel: any)
  local app = { ui = ui, noctalia = noctalia, panel = panel }

  app.PANEL_WIDTH = 920
  app.PANEL_PADDING = 28 -- the host panel's padding, both sides
  -- What the scroll area keeps for its scrollbar. Measured from a
  -- screenshot: tiles sized for the full width came out ~5px narrower each
  -- (pictures shrink to fit; buttons, like their captions, don't).
  app.SCROLL_GUTTER = 20
  -- The width inside the scroll area, for anything drawn in it.
  app.CONTENT_WIDTH = app.PANEL_WIDTH - app.PANEL_PADDING - app.SCROLL_GUTTER
  app.COLUMNS = 4
  app.COLUMN_GAP = 12
  -- Corner radii, at Noctalia's corner scale (see refreshTheme).
  app.BASE_TILE_RADIUS = 12
  app.BASE_SMALL_RADIUS = 8
  app.TILE_RADIUS = app.BASE_TILE_RADIUS
  app.SMALL_RADIUS = app.BASE_SMALL_RADIUS

  -- A button grows to fit its text (it has no way to cut it short), so a
  -- caption is shortened to fit `width` first, ending in "…". The width per
  -- character is an estimate (~0.6em), with room for the button's padding.
  function app.fitText(text, width, fontSize)
    local perChar = (fontSize or 12) * 0.6
    local max = math.max(4, math.floor((width - 24) / perChar))
    local n = utf8.len(text) or #text
    if n <= max then
      return text
    end
    local cut = utf8.offset(text, max) or (max + 1)
    return string.sub(text, 1, cut - 1) .. "…"
  end

  -- Follow Noctalia's corner rounding (read on each open, so a change in
  -- its settings shows the next time the panel opens).
  function app.refreshTheme()
    local scale = Theme.cornerScale(noctalia)
    app.TILE_RADIUS = math.floor(app.BASE_TILE_RADIUS * scale + 0.5)
    app.SMALL_RADIUS = math.floor(app.BASE_SMALL_RADIUS * scale + 0.5)
  end
  app.MAX_TILES = 120 -- beyond this, filtering finds the rest
  app.LIST_TIMEOUT_MS = 60000 -- the first run may make thumbnails
  app.ALL_MONITORS = "*all*"

  local S = {
    images = {}, -- from `wallpicker list`
    monitors = {},
    loading = false,
    errorMessage = "",
    applyingPath = "",

    -- "local" or "online".
    mode = "local",

    -- Online: the provider's saved filters (the app's Filters dialog) apply;
    -- provider, sort, text and page are picked here.
    online = {
      provider = "wallhaven",
      providers = {}, -- { value, label } from `wallpicker search`
      sort = "",
      sorts = {},
      query = "",
      page = 1,
      lastPage = 1,
      total = 0,
      filtersActive = 0, -- this provider's filters that differ from the defaults
      -- The search the pages below belong to; paging stays on it even if the
      -- box has been edited since.
      sessionQuery = "",
      -- Pages of that search already loaded, by number: going back (or forward
      -- again) shows them at once. A new search, sort, provider, filter change
      -- or Refresh clears them.
      pages = {},
      -- Bumped whenever a page is shown: the results' scroll area gets a new key,
      -- so it's rebuilt at the top (scrolls have no "scroll to top").
      scrollRev = 0,
      items = {},
      loading = false,
      error = "",
      applyingKey = "",
      searched = false,
    },

    -- The local wallpaper whose own view replaces the grid (favorite, tags,
    -- trash); nil shows the grid. Opened by clicking a caption, or from its
    -- right-click menu.
    detail = {
      path = nil,
      confirmTrash = false,
      busy = false,
    },
    allTags = {}, -- { name, count }, for the tag chips

    -- The online result whose details replace the grid; nil shows the grid.
    onlineDetail = {
      item = nil,
      data = nil, -- from `wallpicker details`
      preview = nil, -- the sharp image's local copy, from `wallpicker preview`
      loading = false,
      error = "",
    },

    filterText = "",
    favoritesOnly = false,

    -- Local: sort, flat/nested layout, tag AND/OR and Now showing,
    -- remembered across restarts.
    prefs = Library.loadPrefs(noctalia),
    shuffle = {}, -- path -> number, for the "random" sort
    selectedTags = {}, -- tag chips that are on
    roots = {}, -- the wallpaper folders, from `wallpicker list`
    -- Where the nested view is: { root, folder }, or nil for the folder list.
    location = nil,
    locationSet = false,
    localScrollRev = 0, -- bumped on folder changes: back to the top
    focusRev = 0, -- bumped by Ctrl+F: the search field is made again, focused
    -- Index into monitorChoices(): 0 is "All monitors".
    monitorIndex = 0,
    monitorPicked = false,
  }
  app.state = S

  -- Set by panel.luau: app.render (draws everything) and app.config (the
  -- Filters view, config.luau).

  function app.tr(key, subst)
    return noctalia.tr(key, subst)
  end

  -- The wallpicker command to run, or nil when it can't be found.
  function app.command()
    return Command.find(noctalia)
  end

  -- Hand a job to the app: `wallpicker --open PLACE [IMAGE]` shows its
  -- window there (starting it if needed) and the panel closes. PLACE: local,
  -- online, collections, settings, duplicates, or tags with the image.
  function app.openInApp(place, image)
    local cmd = app.command()
    if cmd == nil then
      noctalia.notifyError(app.tr("title"), app.tr("missing_command"))
      return
    end
    local argv = { cmd, "--open", place }
    if image ~= nil then
      table.insert(argv, image)
    end
    -- Detached: no callback.
    noctalia.runAsync(argv)
    app.panel.close()
  end

  -- "All monitors", then each output by name.
  function app.monitorChoices()
    local choices = { app.tr("all_monitors") }
    for _, m in ipairs(S.monitors) do
      table.insert(choices, m.name)
    end
    return choices
  end

  function app.targetMonitor()
    if S.monitorIndex == 0 then
      return nil
    end
    local m = S.monitors[S.monitorIndex]
    return m and m.name
  end

  -- What wallpicker last applied to the target (any monitor when it's "All").
  function app.isCurrent(path)
    for _, m in ipairs(S.monitors) do
      if m.current == path and (app.targetMonitor() == nil or app.targetMonitor() == m.name) then
        return true
      end
    end
    return false
  end

  function app.findImage(path)
    for _, image in ipairs(S.images) do
      if image.path == path then
        return image
      end
    end
    return nil
  end

  -- Monitors showing `path` right now, by name.
  function app.shownOn(path)
    local names = {}
    for _, m in ipairs(S.monitors) do
      if m.current == path then
        table.insert(names, m.name)
      end
    end
    return names
  end

  function app.tileSize()
    local gridWidth = app.CONTENT_WIDTH
    local width = math.floor((gridWidth - app.COLUMN_GAP * (app.COLUMNS - 1)) / app.COLUMNS)
    return width, math.floor(width * 10 / 16)
  end

  -- Rating as the border color: green sfw, yellow sketchy, red nsfw. Fixed
  -- colors, not theme roles, so they mean the same thing in every theme.
  app.PURITY_COLORS = {
    sfw = "#4caf50",
    sketchy = "#fbc02d",
    nsfw = "#e53935",
  }

  app.PURITY_BORDER_WIDTH = 2

  function app.purityBorder(purity)
    return app.PURITY_COLORS[purity] or "outline", app.PURITY_BORDER_WIDTH
  end

  -- Rows don't wrap, so chips are laid out here: each one's width is guessed
  -- from its text (small buttons, 12px text).
  app.CHIP_FONT = 12

  function app.chipWidth(text)
    return (utf8.len(text) or #text) * 7 + 26
  end

  function app.flow(chips, texts)
    local maxWidth = app.CONTENT_WIDTH
    local rows, row, used = {}, {}, 0
    for i, chip in ipairs(chips) do
      local w = app.chipWidth(texts[i])
      if #row > 0 and used + 6 + w > maxWidth then
        table.insert(rows, ui.row({ gap = 6, align = "center" }, row))
        row, used = {}, 0
      end
      used += (if #row > 0 then 6 else 0) + w
      table.insert(row, chip)
    end
    if #row > 0 then
      table.insert(rows, ui.row({ gap = 6, align = "center" }, row))
    end
    return ui.column({ gap = 6, align = "start" }, rows)
  end

  function app.openUrl(url)
    if type(url) == "string" and (string.sub(url, 1, 8) == "https://" or string.sub(url, 1, 7) == "http://") then
      noctalia.runAsync({ "xdg-open", url })
    end
  end

  function app.labels(choices)
    local out = {}
    for _, c in ipairs(choices) do
      table.insert(out, c.label)
    end
    return out
  end

  function app.indexOf(choices, value)
    for i, c in ipairs(choices) do
      if c.value == value then
        return i - 1
      end
    end
    return 0
  end

  function app.monitorSelect()
    return ui.select({
      key = "monitor",
      options = app.monitorChoices(),
      selectedIndex = S.monitorIndex,
      onChange = "onMonitorChange",
      width = 160,
    })
  end

  return app
end

return M
