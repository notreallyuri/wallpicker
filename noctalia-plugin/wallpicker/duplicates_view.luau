--!nonstrict
-- The Duplicates view, in place of the grid: exact copies and the same
-- picture at other sizes (`wallpicker duplicates --json`). The copy to keep
-- is picked already (the largest); click another to keep it instead.
-- "Keep it" runs `wallpicker duplicates keep <kept> <others...>`: the
-- others go to the trash, their favorite and tags move to the kept one.

return function(app)
  local ui, noctalia = app.ui, app.noctalia
  local S = app.state

  S.dups = {
    open = false,
    loading = false,
    groups = nil,
    error = "",
    keep = {}, -- group index -> index of the copy to keep
    busy = false,
  }

  function app.openDuplicates()
    local cmd = app.command()
    if cmd == nil then
      return
    end
    S.dups.open = true
    S.dups.loading = true
    S.dups.groups = nil
    S.dups.error = ""
    S.dups.keep = {}
    app.render()
    noctalia.runAsync({ cmd, "duplicates", "--json" }, function(res)
      S.dups.loading = false
      local d = res and res.exitCode == 0 and noctalia.json.decode(res.stdout)
      if type(d) == "table" then
        S.dups.groups = d
      else
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        S.dups.error = app.tr("dups_failed") .. (if why ~= "" then ": " .. why else "")
      end
      app.render()
    end, app.LIST_TIMEOUT_MS)
  end

  function app.closeDuplicates()
    S.dups.open = false
    app.render()
  end

  local function dropGroup(index)
    table.remove(S.dups.groups, index)
    S.dups.keep = {}
    app.render()
  end

  local function resolve(index)
    local g = S.dups.groups[index]
    local cmd = app.command()
    if g == nil or cmd == nil or S.dups.busy then
      return
    end
    local k = S.dups.keep[index] or 1
    local argv = { cmd, "duplicates", "keep", g.items[k].path }
    -- Only the copies on screen: any beyond them stay as they are.
    for i, item in ipairs(g.items) do
      if i ~= k and i <= app.COLUMNS then
        table.insert(argv, item.path)
      end
    end
    S.dups.busy = true
    app.render()
    noctalia.runAsync(argv, function(res)
      S.dups.busy = false
      if res ~= nil and res.exitCode == 0 then
        noctalia.notify(app.tr("title"), app.tr("dups_done", { n = #argv - 4 }))
        dropGroup(index)
      else
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        noctalia.notifyError(app.tr("trash_failed"), if why ~= "" then why else nil)
        app.render()
      end
      app.loadLibrary()
    end, app.LIST_TIMEOUT_MS)
  end

  local function copyTile(gi, ci, item)
    local width, height = app.tileSize()
    local kept = (S.dups.keep[gi] or 1) == ci
    local image = app.findImage(item.path)
    local pick = function()
      S.dups.keep[gi] = ci
      app.render()
    end
    local frame = {
      width = width,
      height = height,
      radius = app.TILE_RADIUS,
      border = if kept then "primary" else "outline",
      borderWidth = if kept then 3 else 1,
      opacity = if kept then 1 else 0.6,
      onClick = pick,
    }
    local picture
    if image and type(image.thumb) == "string" and image.thumb ~= "" then
      frame.path = image.thumb
      frame.fit = "cover"
      picture = ui.image(frame)
    else
      frame.fill = "surface_variant"
      picture = ui.box(frame)
    end
    local name = string.match(item.path, "[^/]+$") or item.path
    return ui.column({ key = `dup_{gi}_{ci}`, gap = 2, align = "center" }, {
      picture,
      ui.button({
        text = app.fitText((if kept then app.tr("dups_keep") else app.tr("dups_trash")) .. " · " .. name, width, 11),
        variant = if kept then "secondary" else "ghost",
        controlSize = "sm",
        fontSize = 11,
        width = width,
        contentAlign = "center",
        onClick = pick,
      }),
      ui.label({
        text = string.format("%d×%d · %.1f MB", item.width or 0, item.height or 0, (item.size or 0) / 1e6),
        fontSize = 11,
        color = "on_surface_variant",
      }),
    })
  end

  function app.duplicatesView()
    local body = {
      ui.row({ gap = 8, align = "center" }, {
        ui.button({ glyph = "chevron-left", text = app.tr("back"), variant = "ghost", onClick = app.closeDuplicates }),
        ui.label({ text = app.tr("dups_title"), fontWeight = "semibold", color = "on_surface", flexGrow = 1 }),
      }),
      ui.label({ text = app.tr("dups_hint"), fontSize = 12, color = "on_surface_variant", maxLines = 2, maxWidth = app.CONTENT_WIDTH }),
    }
    if S.dups.loading then
      table.insert(body, ui.label({ text = app.tr("dups_looking"), color = "on_surface_variant" }))
    elseif S.dups.error ~= "" then
      table.insert(body, ui.label({ text = S.dups.error, color = "error" }))
    elseif S.dups.groups == nil or #S.dups.groups == 0 then
      table.insert(body, ui.label({ text = app.tr("dups_none"), color = "on_surface_variant" }))
    else
      for gi, g in ipairs(S.dups.groups) do
        local row = {}
        for ci, item in ipairs(g.items) do
          -- Rows of four, like the grid.
          if ci <= app.COLUMNS then
            table.insert(row, copyTile(gi, ci, item))
          end
        end
        local extra = #g.items - app.COLUMNS
        table.insert(body, ui.label({
          text = (if g.likeness == "exact" then app.tr("dups_exact") else app.tr("dups_similar"))
            .. (if extra > 0 then "  " .. app.tr("dups_more", { n = extra }) else ""),
          fontWeight = "semibold",
          color = "on_surface",
        }))
        table.insert(body, ui.row({ gap = app.COLUMN_GAP, align = "start" }, row))
        table.insert(body, ui.row({ gap = 8, align = "center", justify = "end" }, {
          ui.button({ key = `dup_skip_{gi}`, text = app.tr("dups_skip"), variant = "ghost", onClick = function()
            dropGroup(gi)
          end }),
          ui.button({
            key = `dup_keep_{gi}`,
            text = app.tr("dups_resolve"),
            variant = "destructive",
            enabled = not S.dups.busy,
            onClick = function()
              resolve(gi)
            end,
          }),
        }))
        table.insert(body, ui.separator({}))
      end
    end
    return ui.column({ gap = 12, align = "stretch" }, body)
  end
end
