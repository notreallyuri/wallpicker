--!nonstrict
-- The Local tab: loading the library, applying, the grid (flat or by folder),
-- tiles, the right-click menu, the controls row, tag chips and Now showing.

local Library = require("./library.luau")

return function(app)
  local ui, noctalia, panel = app.ui, app.noctalia, app.panel
  local S = app.state

  function app.matches(image)
    if S.favoritesOnly and not image.favorite then
      return false
    end
    if not Library.hasTags(image, S.selectedTags, S.prefs.tagMode) then
      return false
    end
    local f = string.lower(noctalia.string.trim(S.filterText))
    if f == "" then
      return true
    end
    if string.find(string.lower(image.name or ""), f, 1, true) then
      return true
    end
    for _, tag in ipairs(image.tags or {}) do
      if string.find(string.lower(tag), f, 1, true) then
        return true
      end
    end
    return false
  end

  function app.loadLibrary()
    if S.loading then
      return
    end
    local cmd = app.command()
    if cmd == nil then
      S.errorMessage = app.tr("missing_command")
      app.render()
      return
    end
    S.loading = true
    S.errorMessage = ""
    app.render()
    noctalia.runAsync({ cmd, "list", "--json", "--thumbs" }, function(res)
      S.loading = false
      if res == nil or res.exitCode ~= 0 or res.stdoutTruncated then
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        S.errorMessage = app.tr("list_failed") .. (if why ~= "" then ": " .. why else "")
        app.render()
        return
      end
      local decoded = noctalia.json.decode(res.stdout)
      if type(decoded) ~= "table" or type(decoded.images) ~= "table" then
        S.errorMessage = app.tr("list_failed")
        app.render()
        return
      end
      S.images = decoded.images
      S.allTags = if type(decoded.all_tags) == "table" then decoded.all_tags else {}
      S.roots = if type(decoded.roots) == "table" then decoded.roots else {}
      for _, image in ipairs(S.images) do
        -- Outside every folder (shouldn't happen): shown at the first one's top.
        if type(image.root) ~= "number" then
          image.root, image.folder = 0, ""
        end
        if S.shuffle[image.path] == nil then
          S.shuffle[image.path] = math.random()
        end
      end
      if not S.locationSet or not Library.exists(S.images, S.roots, S.location) then
        S.location = Library.topOf(S.roots)
        S.locationSet = true
      end
      -- Chips for tags nobody has any more go away.
      local known = {}
      for _, t in ipairs(S.allTags) do
        known[string.lower(t.name)] = true
      end
      local keep = {}
      for _, t in ipairs(S.selectedTags) do
        if known[string.lower(t)] then
          table.insert(keep, t)
        end
      end
      S.selectedTags = keep
      -- The wallpaper on show may be gone (trashed); back to the grid then.
      if S.detail.path ~= nil then
        local still = false
        for _, image in ipairs(S.images) do
          if image.path == S.detail.path then
            still = true
          end
        end
        if not still then
          S.detail.path = nil
        end
      end
      S.monitors = if type(decoded.monitors) == "table" then decoded.monitors else {}
      -- Default to the focused monitor the first time, like the app does.
      if not S.monitorPicked then
        S.monitorIndex = 0
        for i, m in ipairs(S.monitors) do
          if m.focused then
            S.monitorIndex = i
          end
        end
      end
      app.render()
    end, app.LIST_TIMEOUT_MS)
  end

  -- `target`: a monitor name, ALL_MONITORS, or nil for the Monitor select's.
  function app.apply(path, target)
    local cmd = app.command()
    if S.applyingPath ~= "" or cmd == nil then
      return
    end
    local argv = { cmd, "apply", path }
    if target == nil then
      target = app.targetMonitor()
    elseif target == app.ALL_MONITORS then
      target = nil
    end
    if target ~= nil then
      table.insert(argv, "--monitor")
      table.insert(argv, target)
    end
    S.applyingPath = path
    app.render()
    noctalia.runAsync(argv, function(res)
      S.applyingPath = ""
      if res ~= nil and res.exitCode == 0 then
        noctalia.notify(app.tr("title"), app.tr("applied"))
      else
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        noctalia.notifyError(app.tr("apply_failed"), if why ~= "" then why else nil)
      end
      -- Picks up the new "current" wallpaper for the outline.
      app.loadLibrary()
    end, app.LIST_TIMEOUT_MS)
  end

  -- Run a library edit through the CLI, then reload the library.
  function app.edit(args, failKey)
    local cmd = app.command()
    if cmd == nil or S.detail.busy then
      return
    end
    local argv = { cmd }
    for _, a in ipairs(args) do
      table.insert(argv, a)
    end
    S.detail.busy = true
    app.render()
    noctalia.runAsync(argv, function(res)
      S.detail.busy = false
      if res == nil or res.exitCode ~= 0 then
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        noctalia.notifyError(app.tr(failKey), if why ~= "" then why else nil)
      end
      app.loadLibrary()
    end, app.LIST_TIMEOUT_MS)
  end

  function app.setFavorite(path, on)
    if on then
      app.edit({ "favorite", path }, "edit_failed")
    else
      app.edit({ "favorite", path, "--off" }, "edit_failed")
    end
  end

  function app.localMenu(image)
    local fav = image.favorite == true
    local items = {
      { kind = "header", label = image.name or "" },
      { id = "favorite", label = if fav then app.tr("menu_unfavorite") else app.tr("menu_favorite") },
      { id = "tags", label = app.tr("menu_edit_tags") },
      { id = "fit", label = app.tr("menu_fit") },
      { kind = "separator" },
      { kind = "header", label = app.tr("menu_apply_to") },
      { id = "apply_all", label = app.tr("all_monitors") },
    }
    for _, m in ipairs(S.monitors) do
      table.insert(items, { id = "apply_on:" .. m.name, label = m.name })
    end
    for _, item in ipairs({
      { kind = "separator" },
      { id = "show", label = app.tr("menu_show_in_folder") },
      { id = "copy", label = app.tr("menu_copy_path") },
      { kind = "separator" },
      { id = "trash", label = app.tr("menu_trash") },
    }) do
      table.insert(items, item)
    end
    panel.openContextMenu({
      items = items,
      onActivate = "onLocalMenu",
      context = image.path,
      maxVisible = 20,
    })
  end

  function app.tile(image)
    local width, height = app.tileSize()
    local path = image.path
    local current = app.isCurrent(path)
    local on = app.shownOn(path)
    local onClick = function()
      app.apply(path)
    end
    local frame = {
      width = width,
      height = height,
      radius = app.TILE_RADIUS,
      border = if current then "primary" else "outline",
      borderWidth = if current then 3 else 1,
      opacity = if S.applyingPath ~= "" and S.applyingPath ~= path then 0.5 else 1,
      onClick = onClick,
    }
    local picture
    if type(image.thumb) == "string" and image.thumb ~= "" then
      frame.path = image.thumb
      frame.fit = "cover"
      picture = ui.image(frame)
    else
      frame.fill = "surface_variant"
      picture = ui.box(frame)
    end
    local label = image.name or ""
    if image.favorite then
      label = "★ " .. label
    end
    if S.applyingPath == path then
      label = app.tr("applying")
    end
    return ui.column({ key = path, gap = 4, align = "center", flexGrow = 1 }, {
      picture,
      -- The caption is a button: images can't open menus, buttons can.
      ui.button({
        text = app.fitText(label, width, 11),
        variant = if current then "secondary" else "ghost",
        controlSize = "sm",
        fontSize = 11,
        width = width,
        contentAlign = "center",
        onClick = function()
          app.openDetail(path)
        end,
        onRightClick = function()
          app.localMenu(image)
        end,
      }),
      app.tileInfo(image, on),
    })
  end

  -- Under a tile: its size (amber when it's smaller than a monitor it would
  -- go on: it'd be scaled up there), and where it's showing. A hint only.
  local LOW_RES = "#f0a020"
  function app.tileInfo(image, on)
    local parts = {}
    local w, h = tonumber(image.width) or 0, tonumber(image.height) or 0
    if w > 0 and h > 0 then
      local small = {}
      for _, m in ipairs(S.monitors) do
        local target = app.targetMonitor()
        if (target == nil or target == m.name) and (w < (m.width or 0) or h < (m.height or 0)) then
          table.insert(small, m.name)
        end
      end
      local size = `{w}×{h}`
      if #small > 0 then
        table.insert(parts, ui.label({
          text = app.tr("low_res_for", { size = size, monitors = table.concat(small, ", ") }),
          fontSize = 11,
          color = LOW_RES,
          maxLines = 1,
        }))
      else
        table.insert(parts, ui.label({ text = size, fontSize = 11, color = "on_surface_variant", maxLines = 1 }))
      end
    end
    if #on > 0 then
      table.insert(parts, ui.label({ text = app.tr("shown_on", { monitors = table.concat(on, " · ") }), fontSize = 11, color = "primary", maxLines = 1 }))
    end
    if #parts == 0 then
      return ui.box({ height = 0 })
    end
    return ui.row({ gap = 8, align = "center", justify = "center" }, parts)
  end

  function app.filtering()
    return S.favoritesOnly or #S.selectedTags > 0 or noctalia.string.trim(S.filterText) ~= ""
  end

  -- What the grid shows (and Random picks from): matching images, in the
  -- chosen order; in the nested view, only those in the current folder and
  -- below.
  function app.visibleImages()
    local out = {}
    local nested = S.prefs.layout == "nested"
    for _, image in ipairs(Library.sorted(S.images, S.prefs.sort, S.shuffle)) do
      if app.matches(image) and (not nested or Library.inside(image, S.location)) then
        table.insert(out, image)
      end
    end
    return out
  end

  function app.goTo(loc)
    S.location = loc
    S.localScrollRev += 1
    app.render()
  end

  function app.folderTile(entry)
    local width, height = app.tileSize()
    local open = function()
      app.goTo(entry.loc)
    end
    local frame = { width = width, height = height, radius = app.TILE_RADIUS, border = "outline", borderWidth = 1, opacity = 0.55, onClick = open }
    local picture
    local cover = entry.cover
    if cover and type(cover.thumb) == "string" and cover.thumb ~= "" then
      frame.path = cover.thumb
      frame.fit = "cover"
      picture = ui.image(frame)
    else
      frame.fill = "surface_variant"
      picture = ui.box(frame)
    end
    return ui.column({ key = "folder_" .. entry.name, gap = 4, align = "center", flexGrow = 1 }, {
      picture,
      ui.button({
        -- The count stays whole; the name gives way (the glyph takes room).
        text = app.fitText(entry.name, width - 40, 11) .. `  ·  {entry.count}`,
        glyph = "folder",
        variant = "outline",
        controlSize = "sm",
        fontSize = 11,
        width = width,
        contentAlign = "center",
        onClick = open,
      }),
    })
  end

  -- Up, the way here, and "open this folder" (nested view only).
  function app.breadcrumb()
    local parent = Library.parent(S.roots, S.location)
    local row = {
      ui.button({
        key = "up",
        glyph = "arrow-back-up",
        tooltip = app.tr("folder_up"),
        variant = "ghost",
        controlSize = "sm",
        enabled = parent ~= nil,
        onClick = function()
          if parent then
            app.goTo(parent.loc)
          end
        end,
      }),
    }
    local steps = Library.trail(S.roots, S.location, app.tr("all_folders"))
    for i, step in ipairs(steps) do
      if i > 1 then
        table.insert(row, ui.label({ text = "›", color = "on_surface_variant" }))
      end
      local last = i == #steps
      table.insert(row, ui.button({
        key = "crumb_" .. i,
        text = step.label,
        variant = if last then "secondary" else "ghost",
        controlSize = "sm",
        fontSize = 12,
        onClick = function()
          if not last then
            app.goTo(step.loc)
          end
        end,
      }))
    end
    table.insert(row, ui.spacer({ flexGrow = 1 }))
    local here = Library.pathOf(S.roots, S.location)
    if here then
      table.insert(row, ui.button({
        key = "open_here",
        glyph = "folder-open",
        tooltip = app.tr("menu_show_in_folder"),
        variant = "ghost",
        controlSize = "sm",
        onClick = function()
          noctalia.runAsync({ "xdg-open", here })
        end,
      }))
    end
    return ui.row({ gap = 4, align = "center" }, row)
  end

  function app.grid()
    if S.loading and #S.images == 0 then
      return ui.label({ text = app.tr("loading"), color = "on_surface_variant" })
    end
    if S.errorMessage ~= "" then
      return ui.label({ text = S.errorMessage, color = "error" })
    end
    local nested = S.prefs.layout == "nested"
    local folders, shown = {}, {}
    local total = 0
    if nested and not app.filtering() then
      local direct
      folders, direct = Library.contents(Library.sorted(S.images, S.prefs.sort, S.shuffle), S.roots, S.location)
      total = #direct
      shown = table.move(direct, 1, math.min(#direct, app.MAX_TILES), 1, {})
    else
      local all = app.visibleImages()
      total = #all
      shown = table.move(all, 1, math.min(#all, app.MAX_TILES), 1, {})
    end

    local rows = {}
    if nested then
      table.insert(rows, app.breadcrumb())
      if app.filtering() then
        local here = Library.trail(S.roots, S.location, app.tr("all_folders"))
        local name = if #here > 0 then here[#here].label else ""
        table.insert(rows, ui.label({ text = app.tr("searching_in", { folder = name }), fontSize = 12, color = "on_surface_variant" }))
      end
    end
    if #folders == 0 and total == 0 then
      table.insert(rows, ui.label({ text = app.tr("empty"), color = "on_surface_variant" }))
      return ui.column({ gap = 12, align = "stretch" }, rows)
    end

    local cells = {}
    for _, entry in ipairs(folders) do
      table.insert(cells, app.folderTile(entry))
    end
    for _, image in ipairs(shown) do
      table.insert(cells, app.tile(image))
    end
    local row = {}
    for index, cell in ipairs(cells) do
      table.insert(row, cell)
      if #row == app.COLUMNS or index == #cells then
        -- Pad the last row so its tiles keep the same width.
        while #row < app.COLUMNS do
          local w, h = app.tileSize()
          table.insert(row, ui.box({ width = w, height = h, opacity = 0 }))
        end
        table.insert(rows, ui.row({ gap = app.COLUMN_GAP, align = "start" }, row))
        row = {}
      end
    end
    if total > #shown then
      table.insert(rows, ui.label({
        text = app.tr("showing_first", { shown = #shown, total = total }),
        fontSize = 12,
        color = "on_surface_variant",
      }))
    end
    return ui.column({ gap = 12, align = "stretch" }, rows)
  end

  function app.sortChoices()
    local out = {}
    for _, s in ipairs(Library.SORTS) do
      table.insert(out, app.tr("sort_" .. s))
    end
    return out
  end

  function app.randomLocal()
    local pool = app.visibleImages()
    if #pool == 0 or S.applyingPath ~= "" then
      return
    end
    -- Not what's already on the target, when there's anything else.
    local fresh = {}
    for _, image in ipairs(pool) do
      if not app.isCurrent(image.path) then
        table.insert(fresh, image)
      end
    end
    local from = if #fresh > 0 then fresh else pool
    app.apply(from[math.random(#from)].path)
  end

  -- Back to what the target monitor (or every monitor) showed before.
  function app.previousWallpaper()
    local cmd = app.command()
    if cmd == nil or S.applyingPath ~= "" then
      return
    end
    local argv = { cmd, "previous" }
    local target = app.targetMonitor()
    if target ~= nil then
      table.insert(argv, "--monitor")
      table.insert(argv, target)
    end
    S.applyingPath = "*previous*"
    app.render()
    noctalia.runAsync(argv, function(res)
      S.applyingPath = ""
      if res == nil or res.exitCode ~= 0 then
        noctalia.notify(app.tr("title"), app.tr("previous_none"))
      end
      app.loadLibrary()
    end, app.LIST_TIMEOUT_MS)
  end

  function app.localControls()
    local nested = S.prefs.layout == "nested"
    return ui.row({ gap = 8, align = "center" }, {
      ui.input({
        key = `filter_{S.focusRev}`,
        placeholder = app.tr("filter_placeholder"),
        value = S.filterText,
        focus = S.focusRev > 0,
        flexGrow = 1,
        onChange = "onFilterChange",
        onSubmit = "onFilterChange",
      }),
      ui.button({
        key = "favorites",
        text = app.tr("favorites"),
        glyph = "star",
        variant = if S.favoritesOnly then "primary" else "outline",
        onClick = "onToggleFavorites",
      }),
      ui.select({
        key = "local_sort",
        options = app.sortChoices(),
        selectedIndex = (table.find(Library.SORTS, S.prefs.sort) or 1) - 1,
        width = 130,
        onChange = function(index)
          local picked = Library.SORTS[(tonumber(index) or 0) + 1]
          if picked == nil then
            return
          end
          -- Picking Random again reshuffles.
          if picked == "random" then
            S.shuffle = Library.newShuffle(S.images)
          end
          S.prefs.sort = picked
          Library.savePrefs(noctalia, S.prefs)
          S.localScrollRev += 1
          app.render()
        end,
      }),
      ui.button({
        key = "layout",
        text = if nested then app.tr("layout_nested") else app.tr("layout_flat"),
        glyph = if nested then "folders" else "layout-grid",
        tooltip = app.tr("layout_tooltip"),
        variant = "outline",
        onClick = function()
          S.prefs.layout = if nested then "flat" else "nested"
          Library.savePrefs(noctalia, S.prefs)
          S.localScrollRev += 1
          app.render()
        end,
      }),
      -- The app's: copies are compared side by side there.
      ui.button({
        key = "duplicates",
        glyph = "copy",
        tooltip = app.tr("dups_in_app"),
        variant = "ghost",
        onClick = function()
          app.openInApp("duplicates")
        end,
      }),
      ui.button({
        key = "now_toggle",
        glyph = "device-desktop",
        tooltip = if S.prefs.showNow then app.tr("now_hide") else app.tr("now_show"),
        variant = if S.prefs.showNow then "secondary" else "ghost",
        enabled = #S.monitors > 0,
        onClick = function()
          S.prefs.showNow = not S.prefs.showNow
          Library.savePrefs(noctalia, S.prefs)
          app.render()
        end,
      }),
      ui.button({
        key = "previous",
        glyph = "history",
        tooltip = app.tr("previous_tooltip"),
        variant = "ghost",
        enabled = S.applyingPath == "",
        onClick = app.previousWallpaper,
      }),
      ui.button({
        key = "random",
        text = app.tr("random"),
        glyph = "arrows-shuffle",
        tooltip = app.tr("random_tooltip"),
        variant = "outline",
        enabled = S.applyingPath == "" and #S.images > 0,
        onClick = app.randomLocal,
      }),
    })
  end

  -- Your tags as chips: click to filter by them; All/Any decides how several
  -- combine. Nothing when no wallpaper has a tag.
  app.MAX_TAG_CHIPS = 40

  function app.tagChipsRow()
    if #S.allTags == 0 then
      return nil
    end
    local on = {}
    for _, t in ipairs(S.selectedTags) do
      on[string.lower(t)] = true
    end
    local modeText = if S.prefs.tagMode == "any" then app.tr("tags_any") else app.tr("tags_all")
    local chips = {
      ui.button({
        key = "tag_mode",
        text = modeText,
        variant = "ghost",
        controlSize = "sm",
        fontSize = app.CHIP_FONT,
        tooltip = app.tr("tags_mode_tooltip"),
        onClick = function()
          S.prefs.tagMode = if S.prefs.tagMode == "any" then "all" else "any"
          Library.savePrefs(noctalia, S.prefs)
          app.render()
        end,
      }),
    }
    local texts = { modeText }
    -- Selected first, so a collapsed row never hides one that's on.
    local ordered = {}
    for _, t in ipairs(S.allTags) do
      if on[string.lower(t.name)] then
        table.insert(ordered, t)
      end
    end
    for _, t in ipairs(S.allTags) do
      if not on[string.lower(t.name)] then
        table.insert(ordered, t)
      end
    end
    local tagChips, tagTexts = {}, {}
    for i, t in ipairs(ordered) do
      if i > app.MAX_TAG_CHIPS then
        break
      end
      local selected = on[string.lower(t.name)] == true
      local text = `{t.name}  {t.count}`
      table.insert(tagTexts, text)
      table.insert(tagChips, ui.button({
        key = "chip_" .. t.name,
        text = text,
        variant = if selected then "primary" else "outline",
        controlSize = "sm",
        fontSize = app.CHIP_FONT,
        onClick = function()
          if selected then
            local keep = {}
            for _, s in ipairs(S.selectedTags) do
              if string.lower(s) ~= string.lower(t.name) then
                table.insert(keep, s)
              end
            end
            S.selectedTags = keep
          else
            table.insert(S.selectedTags, t.name)
          end
          S.localScrollRev += 1
          app.render()
        end,
      }))
    end
    local tail, tailTexts = {}, {}
    if #S.selectedTags > 0 then
      table.insert(tailTexts, app.tr("tags_clear"))
      table.insert(tail, ui.button({
        key = "tags_clear",
        text = app.tr("tags_clear"),
        glyph = "close",
        variant = "ghost",
        controlSize = "sm",
        fontSize = app.CHIP_FONT,
        onClick = function()
          S.selectedTags = {}
          app.render()
        end,
      }))
    end
    local function toggleButton(text)
      return ui.button({
        key = "tags_expand",
        text = text,
        variant = "ghost",
        controlSize = "sm",
        fontSize = app.CHIP_FONT,
        onClick = function()
          S.prefs.tagsExpanded = not S.prefs.tagsExpanded
          Library.savePrefs(noctalia, S.prefs)
          app.render()
        end,
      })
    end

    -- How many chips fit on one line beside the rest (widths are guessed,
    -- as for every chip row: rows don't wrap by themselves).
    local maxWidth = app.PANEL_WIDTH - app.PANEL_PADDING
    local GAP = 6
    local used = 0
    for _, text in ipairs(texts) do
      used += app.chipWidth(text) + GAP
    end
    for _, text in ipairs(tailTexts) do
      used += app.chipWidth(text) + GAP
    end
    local fits = 0
    for i, text in ipairs(tagTexts) do
      -- Room for "Expand (+N)" while some don't fit.
      local reserve = if i < #tagTexts then app.chipWidth(app.tr("tags_expand", { n = 99 })) + GAP else 0
      if used + app.chipWidth(text) + reserve > maxWidth then
        break
      end
      used += app.chipWidth(text) + GAP
      fits = i
    end
    local all = fits >= #tagChips

    if S.prefs.tagsExpanded and not all then
      for i, chip in ipairs(tagChips) do
        table.insert(chips, chip)
        table.insert(texts, tagTexts[i])
      end
      for i, node in ipairs(tail) do
        table.insert(chips, node)
        table.insert(texts, tailTexts[i])
      end
      table.insert(chips, toggleButton(app.tr("tags_collapse")))
      table.insert(texts, app.tr("tags_collapse"))
      return app.flow(chips, texts)
    end
    -- One line: what fits, then Clear and Expand.
    for i = 1, fits do
      table.insert(chips, tagChips[i])
    end
    for _, node in ipairs(tail) do
      table.insert(chips, node)
    end
    if not all then
      table.insert(chips, toggleButton(app.tr("tags_expand", { n = #tagChips - fits })))
    end
    return ui.row({ gap = GAP, align = "center" }, chips)
  end

  -- What each monitor shows now; click one to make it the target (again for
  -- all monitors). Hidden unless switched on with the monitor button.
  app.NOW_W, app.NOW_H = 112, 63

  function app.nowShowing()
    if #S.monitors == 0 or not S.prefs.showNow then
      return nil
    end
    local cards = { ui.label({ text = app.tr("now_showing"), fontSize = 12, color = "on_surface_variant" }) }
    for i, m in ipairs(S.monitors) do
      local picked = S.monitorIndex == i
      local select = function()
        S.monitorIndex = if picked then 0 else i
        S.monitorPicked = true
        app.render()
      end
      local image = if m.current then app.findImage(m.current) else nil
      local frame = {
        width = app.NOW_W,
        height = app.NOW_H,
        radius = app.SMALL_RADIUS,
        border = if picked then "primary" else "outline",
        borderWidth = if picked then 2 else 1,
        onClick = select,
      }
      local picture
      local thumb = if image and type(image.thumb) == "string" and image.thumb ~= "" then image.thumb else nil
      if thumb then
        frame.path = thumb
        frame.fit = "cover"
        picture = ui.image(frame)
      else
        frame.fill = "surface_variant"
        picture = ui.box(frame)
      end
      table.insert(cards, ui.column({ key = "now_" .. m.name, gap = 2, align = "center" }, {
        picture,
        ui.button({
          text = app.fitText(m.name, app.NOW_W, 11),
          variant = if picked then "secondary" else "ghost",
          controlSize = "sm",
          fontSize = 11,
          width = app.NOW_W,
          contentAlign = "center",
          tooltip = if image then image.name else app.tr("now_unknown"),
          onClick = select,
        }),
      }))
    end
    return ui.row({ gap = 12, align = "center" }, cards)
  end
end
