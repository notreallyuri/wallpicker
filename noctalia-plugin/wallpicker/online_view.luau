--!nonstrict
-- The Online tab: searching (with remembered pages), applying and
-- downloading results, their grid and menu, the controls row and the pager.

return function(app)
  local ui, noctalia, panel = app.ui, app.noctalia, app.panel
  local S = app.state

  -- Show a loaded page's results, from the top.
  function app.showPage(d)
    S.online.scrollRev += 1
    S.online.items = d.items
    S.online.providers = d.providers or S.online.providers
    S.online.sort = d.sort or ""
    S.online.sorts = d.sorts or {}
    S.online.page = tonumber(d.page) or 1
    S.online.lastPage = tonumber(d.last_page) or 1
    S.online.filtersActive = tonumber(d.filters_active) or 0
    S.online.total = tonumber(d.total) or 0
  end

  -- Fetch one page of `query`. `fresh` forgets the pages loaded so far.
  function app.fetchPage(page, query, sort, fresh)
    local cmd = app.command()
    if cmd == nil then
      S.online.error = app.tr("missing_command")
      app.render()
      return
    end
    if S.online.loading then
      return
    end
    if fresh then
      S.online.pages = {}
      S.online.sessionQuery = query
    end
    S.online.loading = true
    S.online.error = ""
    S.onlineDetail.item = nil
    app.render()
    local argv = { cmd, "search", S.online.provider }
    if query ~= "" then
      table.insert(argv, query)
    end
    table.insert(argv, "--page")
    table.insert(argv, tostring(page))
    if sort ~= nil then
      -- Saved for the app too: it's the provider's sort, wherever it's picked.
      table.insert(argv, "--sort")
      table.insert(argv, sort)
    end
    table.insert(argv, "--json")
    table.insert(argv, "--thumbs")
    local provider = S.online.provider
    noctalia.runAsync(argv, function(res)
      S.online.loading = false
      S.online.searched = true
      if res == nil or res.exitCode ~= 0 or res.stdoutTruncated then
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        S.online.error = app.tr("search_failed") .. (if why ~= "" then ": " .. why else "")
        S.online.items = {}
        app.render()
        return
      end
      local d = noctalia.json.decode(res.stdout)
      if type(d) ~= "table" or type(d.items) ~= "table" then
        S.online.error = app.tr("search_failed")
        app.render()
        return
      end
      -- Kept only if it still belongs to the current search.
      if provider == S.online.provider and query == S.online.sessionQuery then
        S.online.pages[tonumber(d.page) or page] = d
      end
      app.showPage(d)
      app.render()
    end, app.LIST_TIMEOUT_MS)
  end

  -- A new search: the text in the box, from page 1 (or `page`), optionally
  -- with a new sort.
  function app.searchOnline(page, sort)
    app.fetchPage(page or 1, noctalia.string.trim(S.online.query), sort, true)
  end

  -- Another page of the current search; a page seen before shows at once.
  function app.goToPage(page)
    if S.online.loading then
      return
    end
    local cached = S.online.pages[page]
    if cached ~= nil then
      S.onlineDetail.item = nil
      S.online.error = ""
      app.showPage(cached)
      app.render()
      return
    end
    app.fetchPage(page, S.online.sessionQuery, nil, false)
  end

  -- The current page again, from the provider; the other pages are forgotten.
  function app.refreshPage()
    app.fetchPage(S.online.page, S.online.sessionQuery, nil, true)
  end

  function app.applyOnline(item)
    local cmd = app.command()
    if S.online.applyingKey ~= "" or cmd == nil then
      return
    end
    local argv = { cmd, "apply-online", item.handle }
    local target = app.targetMonitor()
    if target ~= nil then
      table.insert(argv, "--monitor")
      table.insert(argv, target)
    end
    S.online.applyingKey = item.key
    app.render()
    noctalia.runAsync(argv, function(res)
      S.online.applyingKey = ""
      if res ~= nil and res.exitCode == 0 then
        item.downloaded = true
        noctalia.notify(app.tr("title"), app.tr("applied"))
      else
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        noctalia.notifyError(app.tr("apply_failed"), if why ~= "" then why else nil)
      end
      app.render()
      -- The download is now in the library.
      app.loadLibrary()
    end, app.LIST_TIMEOUT_MS)
  end

  function app.downloadOnly(item)
    local cmd = app.command()
    if cmd == nil or S.online.applyingKey ~= "" then
      return
    end
    S.online.applyingKey = item.key
    app.render()
    noctalia.runAsync({ cmd, "apply-online", item.handle, "--download-only" }, function(res)
      S.online.applyingKey = ""
      if res ~= nil and res.exitCode == 0 then
        item.downloaded = true
        noctalia.notify(app.tr("title"), app.tr("downloaded"))
      else
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        noctalia.notifyError(app.tr("download_failed"), if why ~= "" then why else nil)
      end
      app.render()
      app.loadLibrary()
    end, app.LIST_TIMEOUT_MS)
  end

  -- A result shown in Online, by key.
  function app.findOnline(key)
    for _, item in ipairs(S.online.items) do
      if item.key == key then
        return item
      end
    end
    return nil
  end

  function app.onlineMenu(item)
    panel.openContextMenu({
      items = {
        { kind = "header", label = `{item.resolution or ""}  {item.likes or ""}` },
        { id = "apply", label = app.tr("menu_apply") },
        { id = "details", label = app.tr("menu_details") },
        { id = "download", label = app.tr("menu_download"), enabled = not item.downloaded },
        { kind = "separator" },
        { id = "open", label = app.tr("menu_open_page"), enabled = type(item.page_url) == "string" and item.page_url ~= "" },
      },
      onActivate = "onOnlineMenu",
      context = item.key,
    })
  end

  function app.onlineTile(item)
    local width, height = app.tileSize()
    local busy = S.online.applyingKey == item.key
    local border, borderWidth = app.purityBorder(item.purity)
    local frame = {
      width = width,
      height = height,
      radius = app.TILE_RADIUS,
      border = border,
      borderWidth = borderWidth,
      opacity = if S.online.applyingKey ~= "" and not busy then 0.5 else 1,
      onClick = function()
        app.applyOnline(item)
      end,
    }
    local picture
    if type(item.thumb) == "string" and item.thumb ~= "" then
      frame.path = item.thumb
      frame.fit = "cover"
      picture = ui.image(frame)
    else
      frame.fill = "surface_variant"
      picture = ui.box(frame)
    end
    local caption = `{item.resolution or ""}  {item.likes or ""}`
    if item.downloaded then
      caption = "✓ " .. caption
    end
    if busy then
      caption = app.tr("applying")
    end
    return ui.column({ key = item.key, gap = 4, align = "center", flexGrow = 1 }, {
      picture,
      ui.button({
        text = app.fitText(caption, width, 11),
        variant = if item.downloaded then "secondary" else "ghost",
        controlSize = "sm",
        fontSize = 11,
        width = width,
        contentAlign = "center",
        onClick = function()
          app.openOnlineDetail(item)
        end,
        onRightClick = function()
          app.onlineMenu(item)
        end,
      }),
    })
  end

  function app.onlineGrid()
    if S.online.loading and #S.online.items == 0 then
      return ui.label({ text = app.tr("loading"), color = "on_surface_variant" })
    end
    if S.online.error ~= "" then
      return ui.label({ text = S.online.error, color = "error" })
    end
    if S.online.searched and #S.online.items == 0 then
      return ui.label({ text = app.tr("empty"), color = "on_surface_variant" })
    end
    local rows = {}
    local row = {}
    for index, item in ipairs(S.online.items) do
      table.insert(row, app.onlineTile(item))
      if #row == app.COLUMNS or index == #S.online.items then
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

  -- Konachan and yande.re match exact tag names only: the real tags what's
  -- typed may mean (`wallpicker tags`). One lookup at a time; if the text
  -- changed meanwhile, it looks again when that one answers.
  S.online.suggest = { items = {}, running = false, asked = nil }
  function app.suggestTags()
    local sg = S.online.suggest
    local typed = noctalia.string.trim(S.online.query)
    local cmd = app.command()
    if S.online.provider == "wallhaven" or typed == "" or cmd == nil then
      if #sg.items > 0 then
        sg.items = {}
        app.render()
      end
      return
    end
    if sg.running then
      return
    end
    sg.running = true
    sg.asked = typed
    noctalia.runAsync({ cmd, "tags", S.online.provider, typed, "--json" }, function(res)
      sg.running = false
      local d = res and res.exitCode == 0 and noctalia.json.decode(res.stdout)
      if noctalia.string.trim(S.online.query) ~= sg.asked then
        app.suggestTags()
        return
      end
      sg.items = if type(d) == "table" and not d.exact and type(d.suggestions) == "table" then d.suggestions else {}
      app.render()
    end, app.LIST_TIMEOUT_MS)
  end

  function app.tagSuggestionsRow()
    local items = S.online.suggest.items
    if S.mode ~= "online" or S.online.provider == "wallhaven" or #items == 0 then
      return nil
    end
    local chips, texts = { ui.label({ text = app.tr("tags_suggest"), fontSize = 12, color = "on_surface_variant" }) }, { app.tr("tags_suggest") }
    for i, t in ipairs(items) do
      if i > 8 then
        break
      end
      local text = app.fitText(`{t.name}  {t.count}` .. (if t.kind ~= "general" then " · " .. t.kind else ""), 320, app.CHIP_FONT)
      table.insert(texts, text)
      table.insert(chips, ui.button({
        key = "suggest_" .. t.name,
        text = text,
        variant = "outline",
        controlSize = "sm",
        fontSize = app.CHIP_FONT,
        onClick = function()
          S.online.query = t.applied
          S.online.inputRev = (S.online.inputRev or 0) + 1
          S.online.suggest.items = {}
          app.searchOnline(1)
        end,
      }))
    end
    return app.flow(chips, texts)
  end

  function app.onlineControls()
    -- Until the first search answers, the provider list is just the default.
    local providers = if #S.online.providers > 0 then S.online.providers else { { value = "wallhaven", label = "Wallhaven" } }
    return ui.row({ gap = 8, align = "center" }, {
      ui.select({
        key = "provider",
        options = app.labels(providers),
        selectedIndex = app.indexOf(providers, S.online.provider),
        onChange = "onProviderChange",
        width = 150,
      }),
      ui.input({
        key = `online_query_{S.online.provider}_{S.online.inputRev or 0}_{S.focusRev}`,
        focus = S.focusRev > 0,
        placeholder = app.tr("search_placeholder"),
        value = S.online.query,
        flexGrow = 1,
        onChange = "onOnlineQueryChange",
        onSubmit = "onOnlineQuerySubmit",
      }),
      ui.select({
        key = `sort_{S.online.provider}`,
        options = app.labels(S.online.sorts),
        selectedIndex = app.indexOf(S.online.sorts, S.online.sort),
        onChange = "onSortChange",
        width = 140,
        enabled = #S.online.sorts > 0,
      }),
      ui.button({
        key = "filters",
        text = if S.online.filtersActive > 0 then `{app.tr("filters")} · {S.online.filtersActive}` else app.tr("filters"),
        glyph = "filter",
        variant = if S.online.filtersActive > 0 then "secondary" else "outline",
        onClick = function()
          local providers = if #S.online.providers > 0 then S.online.providers else {}
          local label = S.online.provider
          for _, p in ipairs(providers) do
            if p.value == S.online.provider then
              label = p.label
            end
          end
          app.config.openFilters(S.online.provider, label)
        end,
      }),
      ui.button({ text = app.tr("search"), variant = "primary", onClick = "onOnlineSearch", enabled = not S.online.loading }),
    })
  end

  function app.onlinePager()
    return ui.row({ align = "center", justify = "center", gap = 8 }, {
      ui.button({
        key = "prev_page",
        glyph = "chevron-left",
        variant = "ghost",
        onClick = "onPrevPage",
        enabled = not S.online.loading and S.online.page > 1,
      }),
      ui.label({
        text = (if S.online.loading then app.tr("loading") else `{S.online.page} / {S.online.lastPage}`)
          .. (if S.online.total > 0 then `  ·  {S.online.total}` else ""),
        color = "on_surface_variant",
      }),
      ui.button({
        key = "next_page",
        glyph = "chevron-right",
        variant = "ghost",
        onClick = "onNextPage",
        enabled = not S.online.loading and S.online.page < S.online.lastPage,
      }),
    })
  end
end
