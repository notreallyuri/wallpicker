--!nonstrict
-- The Collections mode: your Wallhaven collections (private ones too, with
-- the API key), shown like online results: click to apply, the right-click
-- menu, details. Opening one needs your Wallhaven username, set in the app
-- (Settings → Wallhaven): the API doesn't say whose key it is.
--   wallpicker collections --json                 -> the list
--   wallpicker collection <id> --page N --json --thumbs -> a page of one

return function(app)
  local ui, noctalia = app.ui, app.noctalia
  local S = app.state

  S.collections = {
    list = nil, -- { id, label, count, public }
    loading = false,
    error = "",
    chosen = nil, -- the id shown
    items = {},
    page = 1,
    lastPage = 1,
    total = 0,
    pageLoading = false,
    scrollRev = 0,
  }
  local C = S.collections

  local function failure(res, key)
    local why = res and noctalia.string.trim(res.stderr or "") or ""
    return app.tr(key) .. (if why ~= "" then ": " .. why else "")
  end

  function app.fetchCollection(page)
    local cmd = app.command()
    if cmd == nil or C.chosen == nil or C.pageLoading then
      return
    end
    C.pageLoading = true
    C.error = ""
    S.onlineDetail.item = nil
    app.render()
    local id = C.chosen
    noctalia.runAsync({ cmd, "collection", tostring(id), "--page", tostring(page), "--json", "--thumbs" }, function(res)
      C.pageLoading = false
      if C.chosen ~= id then
        return
      end
      local d = res and res.exitCode == 0 and noctalia.json.decode(res.stdout)
      if type(d) == "table" and type(d.items) == "table" then
        C.items = d.items
        C.page = tonumber(d.page) or 1
        C.lastPage = tonumber(d.last_page) or 1
        C.total = tonumber(d.total) or 0
        C.scrollRev += 1
      else
        C.items = {}
        C.error = failure(res, "collection_failed")
      end
      app.render()
    end, app.LIST_TIMEOUT_MS)
  end

  function app.loadCollections()
    local cmd = app.command()
    if cmd == nil or C.loading then
      return
    end
    C.loading = true
    C.error = ""
    app.render()
    noctalia.runAsync({ cmd, "collections", "--json" }, function(res)
      C.loading = false
      local d = res and res.exitCode == 0 and noctalia.json.decode(res.stdout)
      if type(d) ~= "table" then
        C.list = {}
        C.error = failure(res, "collections_failed")
        app.render()
        return
      end
      C.list = d
      -- Keep the one shown if it's still there, else the first.
      local still = false
      for _, c in ipairs(d) do
        if c.id == C.chosen then
          still = true
        end
      end
      if not still then
        C.chosen = if d[1] then d[1].id else nil
      end
      if C.chosen ~= nil then
        app.fetchCollection(1)
      else
        app.render()
      end
    end, app.LIST_TIMEOUT_MS)
  end

  function app.collectionsControls()
    local row = {}
    if C.loading and C.list == nil then
      table.insert(row, ui.label({ text = app.tr("loading"), color = "on_surface_variant" }))
    end
    for _, c in ipairs(C.list or {}) do
      local on = c.id == C.chosen
      local text = `{c.label}  {c.count}` .. (if c.public then "" else "  · " .. app.tr("private"))
      table.insert(row, ui.button({
        key = "collection_" .. c.id,
        text = text,
        variant = if on then "primary" else "outline",
        controlSize = "sm",
        fontSize = app.CHIP_FONT,
        onClick = function()
          if not on then
            C.chosen = c.id
            app.fetchCollection(1)
          end
        end,
      }))
    end
    table.insert(row, ui.spacer({ flexGrow = 1 }))
    table.insert(row, ui.button({ text = app.tr("refresh"), glyph = "refresh", variant = "outline", controlSize = "sm", enabled = not C.loading, onClick = app.loadCollections }))
    return ui.row({ gap = 6, align = "center" }, row)
  end

  function app.collectionsGrid()
    if C.error ~= "" then
      return ui.label({ text = C.error, color = "error", maxLines = 3, maxWidth = app.CONTENT_WIDTH })
    end
    if C.list ~= nil and #C.list == 0 then
      return ui.label({ text = app.tr("collections_none"), color = "on_surface_variant" })
    end
    if C.pageLoading and #C.items == 0 then
      return ui.label({ text = app.tr("loading"), color = "on_surface_variant" })
    end
    local rows, row = {}, {}
    for index, item in ipairs(C.items) do
      table.insert(row, app.onlineTile(item))
      if #row == app.COLUMNS or index == #C.items then
        while #row < app.COLUMNS do
          local w, h = app.tileSize()
          table.insert(row, ui.box({ width = w, height = h, opacity = 0 }))
        end
        table.insert(rows, ui.row({ gap = app.COLUMN_GAP, align = "start" }, row))
        row = {}
      end
    end
    return ui.column({ gap = 12, align = "stretch" }, rows)
  end

  function app.collectionsPager()
    return ui.row({ align = "center", justify = "center", gap = 8 }, {
      ui.button({ key = "col_prev", glyph = "chevron-left", variant = "ghost", enabled = not C.pageLoading and C.page > 1, onClick = function()
        app.fetchCollection(C.page - 1)
      end }),
      ui.label({
        text = if C.pageLoading then app.tr("loading") else `{C.page} / {C.lastPage}  ·  {C.total}`,
        color = "on_surface_variant",
      }),
      ui.button({ key = "col_next", glyph = "chevron-right", variant = "ghost", enabled = not C.pageLoading and C.page < C.lastPage, onClick = function()
        app.fetchCollection(C.page + 1)
      end }),
    })
  end
end
