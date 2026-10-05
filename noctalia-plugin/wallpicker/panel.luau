--!nonstrict
-- The wallpicker library in a Noctalia panel.
--
-- Everything goes through the `wallpicker` command, so the library, favorites,
-- tags and saved state are the app's own, and applying uses whichever backend
-- (hyprpaper or Noctalia) is picked in wallpicker's settings:
--   wallpicker list --json --thumbs           -> monitors and images
--   wallpicker apply <path> --monitor <name>  -> apply and record it
--   wallpicker search <provider> [query] --page N [--sort S] --json --thumbs
--                                             -> a page of online results
--   wallpicker apply-online <handle> --monitor <name>
--                                             -> download, apply, record
--   wallpicker favorite <path> [--off]
--   wallpicker tag <path> add|remove <tag>
--   wallpicker trash <path>                   -> library edits, like the app
--   wallpicker details <handle>               -> an online result's tags, artist,
--                                                uploader and source
--   wallpicker preview <handle>               -> its sharpest image, cached
--
-- The parts:
--   shared.luau        state, layout constants, small helpers
--   local_view.luau    the Local tab (grid, folders, tags, Now showing, menu)
--   local_detail.luau  a local wallpaper's view (favorite, tags, trash)
--   online_view.luau   the Online tab (search, pages, grid, menu)
--   online_detail.luau an online result's details
--   duplicates_view.luau copies and resized versions: keep one, trash the rest
--   collections_view.luau your Wallhaven collections
--   config.luau        the Filters and Settings views
--   library.luau       sorting, tag matching, the folder tree, preferences
--   theme.luau         Noctalia's corner rounding, for thumbnails
-- This file wires them up, draws the frame and holds the host's callbacks.

local Shared = require("./shared.luau")
local Config = require("./config.luau")
local Library = require("./library.luau")

local app = Shared.new(ui, noctalia, panel)
local S = app.state
require("./local_view.luau")(app)
require("./local_detail.luau")(app)
require("./online_view.luau")(app)
require("./online_detail.luau")(app)
require("./duplicates_view.luau")(app)
require("./collections_view.luau")(app)

app.config = Config.new({
  ui = ui,
  noctalia = noctalia,
  tr = app.tr,
  command = app.command,
  width = app.PANEL_WIDTH - app.PANEL_PADDING,
  flow = app.flow,
  monitors = function()
    return S.monitors
  end,
  tags = function()
    return S.allTags
  end,
  render = function()
    app.render()
  end,
  openApp = function()
    onOpenApp()
  end,
  onClosed = function(changed)
    if changed.library or changed.applied then
      app.loadLibrary()
    end
    if S.mode == "online" and (changed.filters or changed.online) then
      app.searchOnline(1)
    else
      if changed.filters or changed.online then
        -- Searched again when the Online tab opens.
        S.online.searched = false
      end
      app.render()
    end
  end,
})

app.render = function()
  local isLocal = S.mode == "local"
  local isCollections = S.mode == "collections"
  -- An online result's details, in the tab it was opened from.
  local showDetail = S.onlineDetail.item ~= nil and S.onlineDetail.from == S.mode
  if app.config.isOpen() then
    panel.render(ui.column({ flexGrow = 1, gap = 12, align = "stretch" }, {
      ui.row({ align = "center", gap = 8 }, {
        ui.label({ text = app.tr("title"), fontSize = 16, fontWeight = "bold", color = "on_surface", flexGrow = 1 }),
        ui.button({ glyph = "close", variant = "ghost", onClick = "onCloseClicked" }),
      }),
      ui.scroll({ flexGrow = 1, gap = 12 }, { app.config.view() }),
    }))
    return
  end
  panel.render(ui.column({ flexGrow = 1, gap = 12, align = "stretch" }, {
    ui.row({ align = "center", gap = 8 }, {
      ui.label({ text = app.tr("title"), fontSize = 16, fontWeight = "bold", color = "on_surface" }),
      ui.row({ gap = 4, align = "center", flexGrow = 1 }, {
        ui.button({ key = "mode_local", text = app.tr("mode_local"), variant = if isLocal then "primary" else "ghost", onClick = "onModeLocal" }),
        ui.button({ key = "mode_online", text = app.tr("mode_online"), variant = if S.mode == "online" then "primary" else "ghost", onClick = "onModeOnline" }),
        ui.button({ key = "mode_collections", text = app.tr("mode_collections"), variant = if isCollections then "primary" else "ghost", onClick = "onModeCollections" }),
      }),
      ui.label({ text = app.tr("monitor"), fontSize = 12, color = "on_surface_variant" }),
      app.monitorSelect(),
      ui.button({ glyph = "refresh", tooltip = app.tr("refresh"), variant = "ghost", onClick = "onRefresh", enabled = not S.loading and not S.online.loading }),
      ui.button({ text = app.tr("open_app"), variant = "outline", onClick = "onOpenApp" }),
      ui.button({ glyph = "settings", tooltip = app.tr("settings_title"), variant = "ghost", onClick = function()
        app.config.openSettings()
      end }),
      ui.button({ glyph = "close", variant = "ghost", onClick = "onCloseClicked" }),
    }),
    if isLocal then app.localControls() elseif isCollections then app.collectionsControls() else app.onlineControls(),
    (if isLocal and S.detail.path == nil and not S.dups.open then app.tagChipsRow() else app.tagSuggestionsRow()) or ui.box({ height = 0 }),
    (if isLocal and S.detail.path == nil and not S.dups.open then app.nowShowing() else nil) or ui.box({ height = 0 }),
    ui.scroll({
      key = if isLocal then `scroll_local_{S.localScrollRev}`
        elseif isCollections then `scroll_collections_{S.collections.scrollRev}`
        else `scroll_online_{S.online.scrollRev}`,
      flexGrow = 1,
      gap = 12,
    }, {
      if isLocal then (if S.dups.open then app.duplicatesView() elseif S.detail.path ~= nil then app.detailView() else app.grid())
      elseif showDetail then app.onlineDetailView()
      elseif isCollections then app.collectionsGrid()
      else app.onlineGrid(),
    }),
    if isLocal or showDetail then ui.box({ height = 0 }) elseif isCollections then app.collectionsPager() else app.onlinePager(),
  }))
end

-- ------------------------------------------------- the host's callbacks ----
-- Named in plugin.toml or in the views' props (onClick = "onRefresh").

function onLocalMenu(actionId, context)
  local image = app.findImage(context)
  if image == nil then
    return
  end
  if actionId == "apply_all" then
    app.apply(image.path, app.ALL_MONITORS)
  elseif string.sub(actionId, 1, 9) == "apply_on:" then
    app.apply(image.path, string.sub(actionId, 10))
  elseif actionId == "show" then
    -- Selected in the file manager when it can do that, else the folder.
    local cmd = app.command()
    if cmd ~= nil then
      noctalia.runAsync({ cmd, "show-in-folder", image.path })
    end
  elseif actionId == "copy" then
    if noctalia.copyToClipboard(image.path, "text/plain") then
      noctalia.notify(app.tr("title"), app.tr("path_copied"))
    end
  elseif actionId == "favorite" then
    app.setFavorite(image.path, not image.favorite)
  elseif actionId == "tags" or actionId == "fit" then
    app.openDetail(image.path)
  elseif actionId == "trash" then
    -- Never straight away: the wallpaper's view asks first.
    app.openDetail(image.path, true)
  end
end

function onOnlineMenu(actionId, context)
  local item = app.findOnline(context)
  if item == nil then
    return
  end
  if actionId == "apply" then
    app.applyOnline(item)
  elseif actionId == "details" then
    app.openOnlineDetail(item)
  elseif actionId == "download" then
    app.downloadOnly(item)
  elseif actionId == "open" and type(item.page_url) == "string" then
    noctalia.runAsync({ "xdg-open", item.page_url })
  end
end


function onOpen(_context)
  app.refreshTheme()
  app.loadLibrary()
end

function onRefresh()
  if S.mode == "collections" then
    app.loadCollections()
  elseif S.mode == "online" then
    app.refreshPage()
  else
    app.loadLibrary()
  end
end

function onModeLocal()
  S.mode = "local"
  app.render()
end

function onModeCollections()
  S.mode = "collections"
  if S.collections.list == nil and not S.collections.loading then
    app.loadCollections()
  else
    app.render()
  end
end

function onModeOnline()
  S.mode = "online"
  if not S.online.searched and not S.online.loading then
    app.searchOnline(1)
  else
    app.render()
  end
end

function onProviderChange(index, _text)
  local providers = if #S.online.providers > 0 then S.online.providers else { { value = "wallhaven" } }
  local picked = providers[(tonumber(index) or 0) + 1]
  if picked == nil or picked.value == S.online.provider then
    return
  end
  S.online.provider = picked.value
  -- The new provider's own saved sort comes back with the results.
  S.online.sorts = {}
  S.online.items = {}
  app.searchOnline(1)
end

function onSortChange(index, _text)
  local picked = S.online.sorts[(tonumber(index) or 0) + 1]
  if picked == nil or picked.value == S.online.sort then
    return
  end
  app.searchOnline(1, picked.value)
end

function onOnlineQueryChange(value)
  S.online.query = value
  app.suggestTags()
end

function onOnlineQuerySubmit(value)
  S.online.query = value
  app.searchOnline(1)
end

function onOnlineSearch()
  app.searchOnline(1)
end

function onPrevPage()
  if S.online.page > 1 then
    app.goToPage(S.online.page - 1)
  end
end

function onNextPage()
  if S.online.page < S.online.lastPage then
    app.goToPage(S.online.page + 1)
  end
end

function onOpenApp()
  local cmd = app.command()
  if cmd == nil then
    noctalia.notifyError(app.tr("title"), app.tr("missing_command"))
    return
  end
  -- Detached: no callback.
  noctalia.runAsync({ cmd })
  panel.close()
end

function onFilterChange(value)
  S.filterText = value
  app.render()
end

function onToggleFavorites()
  S.favoritesOnly = not S.favoritesOnly
  app.render()
end

function onMonitorChange(index, _text)
  S.monitorIndex = tonumber(index) or 0
  S.monitorPicked = true
  app.render()
end

function onCloseClicked()
  panel.close()
end

-- The chords plugin.toml captures (capture_keys), as written there.
function onKey(chord, pressed)
  if not pressed then
    return
  end
  if chord == "escape" then
    -- Back out of whatever replaced the grid; at the grid, close.
    if app.config.isOpen() then
      app.config.close()
    elseif S.mode ~= "local" and S.onlineDetail.item ~= nil and S.onlineDetail.from == S.mode then
      app.closeOnlineDetail()
    elseif S.mode == "local" and S.dups.open then
      app.closeDuplicates()
    elseif S.mode == "local" and S.detail.path ~= nil then
      app.closeDetail()
    else
      panel.close()
    end
  elseif chord == "ctrl+f" then
    S.focusRev += 1
    app.render()
  elseif chord == "ctrl+r" then
    if S.mode == "local" and not app.config.isOpen() then
      app.randomLocal()
    end
  elseif chord == "alt+up" then
    if S.mode == "local" and S.prefs.layout == "nested" and S.detail.path == nil then
      local parent = Library.parent(S.roots, S.location)
      if parent then
        app.goTo(parent.loc)
      end
    end
  elseif chord == "alt+left" then
    if S.mode == "online" and S.onlineDetail.item == nil then
      onPrevPage()
    end
  elseif chord == "alt+right" then
    if S.mode == "online" and S.onlineDetail.item == nil then
      onNextPage()
    end
  elseif chord == "ctrl+1" then
    onModeLocal()
  elseif chord == "ctrl+2" then
    onModeOnline()
  elseif chord == "ctrl+3" then
    onModeCollections()
  end
end

app.refreshTheme()
app.render()
