--!nonstrict
-- An online result's details, in place of the grid: big preview, stats,
-- artist/uploader/source links and tags, each of which searches.

return function(app)
  local ui, noctalia = app.ui, app.noctalia
  local S = app.state

  -- Search this provider for `query` (a tag, an artist, an uploader).
  function app.searchFor(query)
    if type(query) ~= "string" or query == "" then
      return
    end
    S.online.query = query
    S.online.inputRev = (S.online.inputRev or 0) + 1
    app.searchOnline(1)
  end

  function app.openOnlineDetail(item)
    local cmd = app.command()
    S.onlineDetail.item = item
    -- Shown only in the tab it was opened from.
    S.onlineDetail.from = S.mode
    S.onlineDetail.data = nil
    S.onlineDetail.preview = nil
    S.onlineDetail.error = ""
    S.onlineDetail.loading = cmd ~= nil
    app.render()
    if cmd == nil then
      return
    end
    -- The sharp image (Wallhaven's original can be several MB) loads on its
    -- own, so the tags don't wait for it; the thumbnail shows meanwhile.
    noctalia.runAsync({ cmd, "preview", item.handle }, function(res)
      if S.onlineDetail.item ~= item or res == nil or res.exitCode ~= 0 then
        return
      end
      local d = noctalia.json.decode(res.stdout)
      if type(d) == "table" and type(d.preview) == "string" then
        S.onlineDetail.preview = d.preview
        app.render()
      end
    end, app.LIST_TIMEOUT_MS)
    noctalia.runAsync({ cmd, "details", item.handle }, function(res)
      -- Another result may have been opened meanwhile.
      if S.onlineDetail.item ~= item then
        return
      end
      S.onlineDetail.loading = false
      if res == nil or res.exitCode ~= 0 or res.stdoutTruncated then
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        S.onlineDetail.error = app.tr("details_failed") .. (if why ~= "" then ": " .. why else "")
      else
        local d = noctalia.json.decode(res.stdout)
        if type(d) == "table" then
          S.onlineDetail.data = d
          if d.downloaded then
            item.downloaded = true
          end
        else
          S.onlineDetail.error = app.tr("details_failed")
        end
      end
      app.render()
    end, app.LIST_TIMEOUT_MS)
  end

  function app.closeOnlineDetail()
    S.onlineDetail.item = nil
    S.onlineDetail.data = nil
    app.render()
  end

  function app.previewHeight(item, width)
    local w, h = string.match(item.resolution or "", "^(%d+)x(%d+)$")
    w, h = tonumber(w), tonumber(h)
    local height = if w and h and w > 0 then math.floor(width * h / w) else math.floor(width * 9 / 16)
    return math.min(height, 440)
  end

  function app.onlineDetailView()
    local item = S.onlineDetail.item
    local d = S.onlineDetail.data
    local busy = S.online.applyingKey == item.key
    local width = app.CONTENT_WIDTH
    local preview = S.onlineDetail.preview or item.thumb

    local header = ui.row({ gap = 8, align = "center" }, {
      ui.button({ glyph = "chevron-left", text = app.tr("back"), variant = "ghost", onClick = app.closeOnlineDetail }),
      ui.label({
        text = if d then `{d.provider_label} · {d.title}` else (item.resolution or ""),
        fontWeight = "semibold",
        color = "on_surface",
        flexGrow = 1,
        maxLines = 1,
      }),
      ui.button({ glyph = "external-link", text = app.tr("open_page"), variant = "ghost", onClick = function()
        app.openUrl(item.page_url)
      end }),
      ui.button({
        text = if item.downloaded then app.tr("downloaded") else app.tr("menu_download"),
        variant = "outline",
        enabled = not item.downloaded and S.online.applyingKey == "",
        onClick = function()
          app.downloadOnly(item)
        end,
      }),
      ui.button({
        text = if busy then app.tr("applying") else app.tr("menu_apply"),
        variant = "primary",
        enabled = S.online.applyingKey == "",
        onClick = function()
          app.applyOnline(item)
        end,
      }),
    })

    local picture
    local border, borderWidth = app.purityBorder(item.purity)
    local frame = { width = width, height = app.previewHeight(item, width), radius = app.TILE_RADIUS, border = border, borderWidth = borderWidth }
    if type(preview) == "string" and preview ~= "" then
      frame.path = preview
      frame.fit = "contain"
      picture = ui.image(frame)
    else
      frame.fill = "surface_variant"
      picture = ui.box(frame)
    end

    local body = { header, picture }
    if S.onlineDetail.loading then
      table.insert(body, ui.label({ text = app.tr("loading_details"), color = "on_surface_variant" }))
    elseif S.onlineDetail.error ~= "" then
      table.insert(body, ui.label({ text = S.onlineDetail.error, color = "error" }))
    end
    if d then
      if type(d.stats) == "table" and #d.stats > 0 then
        table.insert(body, ui.label({ text = table.concat(d.stats, "  ·  "), fontSize = 12, color = "on_surface_variant" }))
      end
      -- Artist, uploader and source: where to find more.
      local people, texts = {}, {}
      for _, a in ipairs(d.artists or {}) do
        local text = (string.gsub(app.tr("more_by"), "{name}", function() return a.name end))
        table.insert(people, ui.button({ key = "artist_" .. a.name, text = text, glyph = "palette", variant = "secondary", controlSize = "sm", fontSize = app.CHIP_FONT, onClick = function()
          app.searchFor(a.query)
        end }))
        table.insert(texts, text .. "   ")
      end
      if type(d.uploader) == "table" and type(d.uploader.query) == "string" then
        local text = (string.gsub(app.tr("more_from"), "{name}", function() return d.uploader.name end))
        table.insert(people, ui.button({ key = "uploader", text = text, glyph = "user", variant = "outline", controlSize = "sm", fontSize = app.CHIP_FONT, onClick = function()
          app.searchFor(d.uploader.query)
        end }))
        table.insert(texts, text .. "   ")
      end
      if type(d.source) == "string" and d.source ~= "" then
        table.insert(people, ui.button({ key = "source", text = app.tr("source"), glyph = "link", variant = "outline", controlSize = "sm", fontSize = app.CHIP_FONT, onClick = function()
          app.openUrl(d.source)
        end }))
        table.insert(texts, app.tr("source") .. "   ")
      end
      if #people > 0 then
        table.insert(body, app.flow(people, texts))
      end
      -- Tags: click one to search for it.
      local tags, tagTexts = {}, {}
      for _, t in ipairs(d.tags or {}) do
        if not t.is_artist then
          table.insert(tags, ui.button({ key = "otag_" .. t.name, text = t.name, variant = "ghost", controlSize = "sm", fontSize = app.CHIP_FONT, onClick = function()
            app.searchFor(t.query)
          end }))
          table.insert(tagTexts, t.name)
        end
      end
      if #tags > 0 then
        table.insert(body, ui.label({ text = app.tr("tags_search_hint"), fontSize = 12, color = "on_surface_variant" }))
        table.insert(body, app.flow(tags, tagTexts))
      end
    end
    return ui.column({ gap = 14, align = "stretch" }, body)
  end
end
