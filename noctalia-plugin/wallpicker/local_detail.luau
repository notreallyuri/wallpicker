--!nonstrict
-- A local wallpaper's own view, in place of the grid: preview, favorite,
-- fit, its tags (edited in the app) and trash, which asks first.

return function(app)
  local ui, noctalia = app.ui, app.noctalia
  local S = app.state

  -- The wallpaper's own fit on any monitor; Automatic uses the monitor's
  -- (Settings). Saved and re-applied where it shows.
  local FITS = { "", "cover", "contain", "tile", "fill" }
  function app.fitRow(image)
    local labels = {}
    for _, f in ipairs(FITS) do
      table.insert(labels, app.tr("fit_" .. (if f == "" then "auto" else f)))
    end
    return ui.row({ gap = 8, align = "center" }, {
      ui.label({ text = app.tr("fit"), fontSize = 12, color = "on_surface_variant", width = 60 }),
      ui.select({
        key = "fit_" .. image.path,
        options = labels,
        selectedIndex = (table.find(FITS, image.fit or "") or 1) - 1,
        width = 180,
        enabled = not S.detail.busy,
        onChange = function(index)
          local f = FITS[(tonumber(index) or 0) + 1]
          if f ~= nil and f ~= (image.fit or "") then
            app.edit({ "fit", image.path, if f == "" then "auto" else f }, "edit_failed")
          end
        end,
      }),
      ui.label({ text = app.tr("fit_hint"), fontSize = 11, color = "on_surface_variant", flexGrow = 1, maxLines = 2 }),
    })
  end

  function app.openDetail(path, confirmTrash)
    S.detail.path = path
    S.detail.confirmTrash = confirmTrash == true
    app.render()
  end

  function app.closeDetail()
    S.detail.path = nil
    S.detail.confirmTrash = false
    app.render()
  end

  -- Its tags; one picks that tag in the grid. Editing them is the app's.
  function app.tagChips(image)
    local chips, texts = {}, {}
    for _, tag in ipairs(image.tags or {}) do
      table.insert(texts, tag)
      table.insert(chips, ui.button({
        key = "tag_" .. tag,
        text = tag,
        variant = "secondary",
        controlSize = "sm",
        fontSize = app.CHIP_FONT,
        tooltip = app.tr("tag_filter_hint"),
        onClick = function()
          S.selectedTags = { tag }
          app.closeDetail()
        end,
      }))
    end
    if #chips == 0 then
      return ui.label({ text = app.tr("no_tags"), color = "on_surface_variant" })
    end
    return app.flow(chips, texts)
  end

  function app.detailView()
    local image = app.findImage(S.detail.path)
    if image == nil then
      return ui.label({ text = app.tr("loading"), color = "on_surface_variant" })
    end
    local path = image.path
    local fav = image.favorite == true
    local trashRow
    if S.detail.confirmTrash then
      trashRow = ui.row({ gap = 8, align = "center" }, {
        ui.label({ text = app.tr("trash_confirm"), color = "error", flexGrow = 1 }),
        ui.button({ text = app.tr("cancel"), variant = "ghost", onClick = function()
          S.detail.confirmTrash = false
          app.render()
        end }),
        ui.button({ text = app.tr("menu_trash_now"), variant = "destructive", enabled = not S.detail.busy, onClick = function()
          S.detail.confirmTrash = false
          app.edit({ "trash", path }, "trash_failed")
        end }),
      })
    else
      trashRow = ui.row({ gap = 8, align = "center" }, {
        ui.label({ text = app.tr("trash_hint"), fontSize = 12, color = "on_surface_variant", flexGrow = 1 }),
        ui.button({ text = app.tr("menu_trash"), variant = "outline", onClick = function()
          S.detail.confirmTrash = true
          app.render()
        end }),
      })
    end
    local width = app.CONTENT_WIDTH
    return ui.column({ gap = 14, align = "stretch" }, {
      ui.row({ gap = 8, align = "center" }, {
        ui.button({ glyph = "chevron-left", text = app.tr("back"), variant = "ghost", onClick = app.closeDetail }),
        ui.label({ text = image.name or "", fontWeight = "semibold", color = "on_surface", flexGrow = 1, maxLines = 1 }),
        ui.button({
          text = if fav then app.tr("menu_unfavorite") else app.tr("menu_favorite"),
          glyph = "star",
          variant = if fav then "primary" else "outline",
          enabled = not S.detail.busy,
          onClick = function()
            app.setFavorite(path, not fav)
          end,
        }),
        ui.button({ text = app.tr("menu_apply"), variant = "primary", enabled = S.applyingPath == "", onClick = function()
          app.apply(path)
        end }),
      }),
      -- The full image, not the grid thumbnail.
      ui.image({ path = path, width = width, height = math.floor(width * 9 / 16), fit = "contain", radius = app.TILE_RADIUS, border = "outline", borderWidth = 1 }),
      app.fitRow(image),
      ui.row({ gap = 8, align = "center" }, {
        ui.label({ text = app.tr("tags"), fontSize = 12, color = "on_surface_variant", flexGrow = 1 }),
        ui.button({ text = app.tr("edit_tags_in_app"), glyph = "external-link", variant = "outline", controlSize = "sm", onClick = function()
          app.openInApp("tags", path)
        end }),
      }),
      app.tagChips(image),
      ui.separator({}),
      trashRow,
    })
  end
end
