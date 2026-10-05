--!nonstrict
-- The panel's Filters (per provider) and Settings views. They read and save
-- through the wallpicker command, so they are the same files as the app's
-- Filters dialog and Settings:
--   wallpicker filters --json                          -> every provider's filters
--   wallpicker filters <provider> set KEY=VALUE | reset
--   wallpicker settings --json / settings set KEY=VALUE
-- Every change is saved as it's made, like in the app.

local M = {}

local LABEL_WIDTH = 170
local SELECT_WIDTH = 280
local TIMEOUT_MS = 30000

-- deps: ui, noctalia, tr, command, render, openApp, width, flow (wrapping
-- chips), monitors() (for their sizes), tags() (the library's, for the
-- slideshow), and onClosed(changed), where
-- changed has `filters`, `library` and `online` set when those need
-- reloading.
function M.new(deps)
  local ui, noctalia, tr = deps.ui, deps.noctalia, deps.tr

  local state = {
    view = nil, -- "filters" or "settings"
    provider = "wallhaven",
    providerLabel = "Wallhaven",
    filters = nil, -- from `wallpicker filters`
    settings = nil, -- from `wallpicker settings`
    loading = false,
    busy = false,
    error = "",
    changed = {},
    inputRev = 0, -- bumped to put text inputs back to the saved values
  }

  local function run(args, onData)
    local cmd = deps.command()
    if cmd == nil then
      state.error = tr("missing_command")
      deps.render()
      return
    end
    local argv = { cmd }
    for _, a in ipairs(args) do
      table.insert(argv, a)
    end
    state.busy = true
    deps.render()
    noctalia.runAsync(argv, function(res)
      state.busy = false
      state.loading = false
      if res == nil or res.exitCode ~= 0 or res.stdoutTruncated then
        -- The first line says what's wrong; the rest is the usage text.
        local why = res and noctalia.string.trim(res.stderr or "") or ""
        why = string.match(why, "^[^\n]*") or why
        state.error = tr("config_failed") .. (if why ~= "" then ": " .. why else "")
      else
        local d = noctalia.json.decode(res.stdout)
        if type(d) == "table" then
          state.error = ""
          onData(d)
        else
          state.error = tr("config_failed")
        end
      end
      state.inputRev += 1
      deps.render()
    end, TIMEOUT_MS)
  end

  local self = {}

  function self.isOpen()
    return state.view ~= nil
  end

  function self.current()
    return state.view
  end

  function self.openFilters(provider, providerLabel)
    state.view = "filters"
    state.provider = provider
    state.providerLabel = providerLabel or provider
    state.filters = nil
    state.loading = true
    state.error = ""
    run({ "filters", "--json" }, function(d)
      state.filters = d
    end)
  end

  function self.openSettings()
    state.view = "settings"
    state.settings = nil
    state.loading = true
    state.error = ""
    run({ "settings", "--json" }, function(d)
      state.settings = d
      self.refreshSlideshow()
    end)
  end

  -- When the slideshow changes next (`wallpicker slideshow status`).
  function self.refreshSlideshow()
    local cmd = deps.command()
    if cmd == nil then
      return
    end
    noctalia.runAsync({ cmd, "slideshow", "status" }, function(res)
      local d = res and res.exitCode == 0 and noctalia.json.decode(res.stdout)
      state.slideshowStatus = if type(d) == "table" then d else nil
      deps.render()
    end, TIMEOUT_MS)
  end

  local function changeNow()
    run({ "slideshow", "tick", "--now" }, function(_)
      state.changed.applied = true
    end)
  end

  function self.close()
    local changed = state.changed
    state.view = nil
    state.changed = {}
    state.error = ""
    deps.onClosed(changed)
  end

  local function setFilter(key, value)
    run({ "filters", state.provider, "set", `{key}={value}` }, function(d)
      state.filters = d
      state.changed.filters = true
    end)
  end

  local function resetFilters()
    run({ "filters", state.provider, "reset" }, function(d)
      state.filters = d
      state.changed.filters = true
    end)
  end

  -- Settings that change what the library shows, or what online finds.
  local LIBRARY_KEYS = { folders = true, recursive = true, thumb_width = true }
  -- (A fit change is re-applied by wallpicker itself; the grid doesn't change.)
  local ONLINE_KEYS = { konachan_site = true }

  local function setSetting(key, value)
    run({ "settings", "set", `{key}={value}` }, function(d)
      state.settings = d
      if string.sub(key, 1, 10) == "slideshow." then
        self.refreshSlideshow()
      end
      if LIBRARY_KEYS[key] then
        state.changed.library = true
      end
      if ONLINE_KEYS[key] then
        state.changed.online = true
      end
    end)
  end

  -- ---------------------------------------------------------- controls ----

  local function bool(b)
    return if b then "true" else "false"
  end

  local function labels(choices)
    local out = {}
    for _, c in ipairs(choices or {}) do
      table.insert(out, c.label)
    end
    return out
  end

  local function indexOf(choices, value)
    for i, c in ipairs(choices or {}) do
      if c.value == value then
        return i - 1
      end
    end
    return 0
  end

  local function section(text)
    return ui.label({ text = text, fontSize = 13, fontWeight = "semibold", color = "on_surface" })
  end

  local function hint(text, color)
    return ui.label({
      text = text,
      fontSize = 12,
      color = color or "on_surface_variant",
      maxLines = 3,
      maxWidth = deps.width,
    })
  end

  local function toggle(key, text, checked, onPick)
    return ui.row({ gap = 8, align = "center" }, {
      ui.toggle({
        key = key,
        checked = checked == true,
        enabled = not state.busy,
        onChange = function(value)
          onPick(value == "true")
        end,
      }),
      ui.label({ text = text, color = "on_surface" }),
    })
  end

  local function choice(key, text, choices, value, onPick)
    return ui.row({ gap = 8, align = "center" }, {
      ui.label({ text = text, width = LABEL_WIDTH, color = "on_surface_variant" }),
      ui.select({
        key = key,
        options = labels(choices),
        selectedIndex = indexOf(choices, value),
        width = SELECT_WIDTH,
        enabled = not state.busy,
        onChange = function(index)
          local c = (choices or {})[(tonumber(index) or 0) + 1]
          if c ~= nil and c.value ~= value then
            onPick(c.value)
          end
        end,
      }),
    })
  end

  -- A text field saved on Enter.
  local function field(key, text, value, placeholder, width, onSave)
    return ui.row({ gap = 8, align = "center" }, {
      ui.label({ text = text, width = LABEL_WIDTH, color = "on_surface_variant" }),
      ui.input({
        key = `{key}_{state.inputRev}`,
        value = value,
        placeholder = placeholder,
        width = width,
        flexGrow = if width then nil else 1,
        enabled = not state.busy,
        onSubmit = function(v)
          v = noctalia.string.trim(v)
          if v ~= "" and v ~= value then
            onSave(v)
          end
        end,
      }),
    })
  end

  local function header(title, extra)
    local row = {
      ui.button({ glyph = "chevron-left", text = tr("done"), variant = "ghost", onClick = self.close }),
      ui.label({ text = title, fontWeight = "semibold", color = "on_surface", flexGrow = 1 }),
    }
    for _, node in ipairs(extra or {}) do
      table.insert(row, node)
    end
    return ui.row({ gap = 8, align = "center" }, row)
  end

  -- ----------------------------------------------------------- filters ----

  -- "1920x1080" -> "1920×1080".
  local function pretty(size)
    return (string.gsub(size, "x", "×"))
  end

  -- What was typed, as WIDTHxHEIGHT, or nil.
  local function typedSize(v)
    v = string.lower(noctalia.string.trim(v))
    v = string.gsub(string.gsub(v, "×", "x"), "%s", "")
    local w, h = string.match(v, "^(%d+)x(%d+)$")
    if w and #w <= 5 and #h <= 5 then
      return v
    end
    return nil
  end

  local function has(list, value)
    for _, v in ipairs(list or {}) do
      if v == value then
        return true
      end
    end
    return false
  end

  -- Chips, several of which can be on; clicking one saves the new list.
  local function chips(keyPrefix, items, picked, onPick)
    local nodes, texts = {}, {}
    for _, item in ipairs(items) do
      local on = has(picked, item.value)
      table.insert(texts, item.label)
      table.insert(nodes, ui.button({
        key = keyPrefix .. item.value,
        text = item.label,
        variant = if on then "primary" else "outline",
        controlSize = "sm",
        fontSize = 12,
        enabled = not state.busy,
        onClick = function()
          local next = {}
          for _, v in ipairs(picked or {}) do
            if v ~= item.value then
              table.insert(next, v)
            end
          end
          if not on then
            table.insert(next, item.value)
          end
          onPick(table.concat(next, ","))
        end,
      }))
    end
    return deps.flow(nodes, texts)
  end

  -- Minimum size: a preset, or any size typed in. Nothing is assumed: small
  -- sizes are fine finds for small or 4:3 screens.
  local function minSize(f, o)
    local presets = o.min_resolutions or {}
    local listed = false
    for _, c in ipairs(presets) do
      if c.value == f.atleast then
        listed = true
      end
    end
    local options = presets
    if not listed and f.atleast ~= "" then
      options = table.clone(presets)
      table.insert(options, 2, { value = f.atleast, label = "≥ " .. pretty(f.atleast) })
    end
    return {
      choice("atleast", tr("f_min_size"), options, f.atleast, function(v)
        setFilter("atleast", v)
      end),
      field("atleast_custom", tr("f_min_other"), "", "1700x900", 160, function(v)
        local size = typedSize(v)
        if size then
          setFilter("atleast", size)
        else
          state.error = tr("f_bad_size")
          deps.render()
        end
      end),
    }
  end

  -- Exact resolutions: your monitors' sizes first (named), the presets, and
  -- any typed in.
  local function exactSizes(f, o)
    local items, seen = {}, {}
    for _, m in ipairs(deps.monitors() or {}) do
      local size = `{m.width}x{m.height}`
      if seen[size] then
        seen[size].label ..= ", " .. m.name
      else
        local item = { value = size, label = `{pretty(size)} · {m.name}` }
        seen[size] = item
        table.insert(items, item)
      end
    end
    for _, c in ipairs(o.resolutions or {}) do
      if not seen[c.value] then
        seen[c.value] = c
        table.insert(items, c)
      end
    end
    for _, size in ipairs(f.resolutions or {}) do
      if not seen[size] then
        seen[size] = true
        table.insert(items, { value = size, label = pretty(size) })
      end
    end
    local function save(list)
      setFilter("resolutions", list)
    end
    return {
      chips("res_", items, f.resolutions, save),
      field("res_custom", tr("f_res_other"), "", "1024x768", 160, function(v)
        local size = typedSize(v)
        if size == nil then
          state.error = tr("f_bad_size")
          deps.render()
        elseif not has(f.resolutions, size) then
          local next = table.clone(f.resolutions or {})
          table.insert(next, size)
          save(table.concat(next, ","))
        end
      end),
    }
  end

  local function wallhavenFilters(d)
    local f, o = d.wallhaven, d.options
    local body = {
      section(tr("f_categories")),
      ui.row({ gap = 24, align = "center" }, {
        toggle("cat_general", tr("f_general"), f.general, function(v)
          setFilter("general", bool(v))
        end),
        toggle("cat_anime", tr("f_anime"), f.anime, function(v)
          setFilter("anime", bool(v))
        end),
        toggle("cat_people", tr("f_people"), f.people, function(v)
          setFilter("people", bool(v))
        end),
      }),
      section(tr("f_purity")),
      ui.row({ gap = 24, align = "center" }, {
        toggle("pur_sfw", "SFW", f.sfw, function(v)
          setFilter("sfw", bool(v))
        end),
        toggle("pur_sketchy", tr("f_sketchy"), f.sketchy, function(v)
          setFilter("sketchy", bool(v))
        end),
        toggle("pur_nsfw", "NSFW", f.nsfw, function(v)
          setFilter("nsfw", bool(v))
        end),
      }),
    }
    if f.nsfw and not d.api_key_set then
      table.insert(body, hint(tr("f_nsfw_needs_key"), "error"))
    end
    table.insert(body, section(tr("f_size")))
    for _, node in ipairs(minSize(f, o)) do
      table.insert(body, node)
    end
    if #(f.resolutions or {}) > 0 and f.atleast ~= "" then
      table.insert(body, hint(tr("f_min_unused")))
    end
    table.insert(body, section(tr("f_exact")))
    for _, node in ipairs(exactSizes(f, o)) do
      table.insert(body, node)
    end
    table.insert(body, section(tr("f_ratios")))
    table.insert(body, chips("ratio_", o.ratios or {}, f.ratios, function(list)
      setFilter("ratios", list)
    end))
    table.insert(body, section(tr("f_toplist")))
    table.insert(body, choice("range", tr("f_range"), o.top_ranges, f.range, function(v)
      setFilter("range", v)
    end))
    table.insert(body, hint(tr("f_range_hint")))
    return body
  end

  local function moebooruFilters(d)
    local f, o = d[state.provider], d.options
    local body = {
      section(tr("f_ratings")),
      ui.row({ gap = 24, align = "center" }, {
        toggle("r_safe", tr("f_safe"), f.safe, function(v)
          setFilter("safe", bool(v))
        end),
        toggle("r_questionable", tr("f_questionable"), f.questionable, function(v)
          setFilter("questionable", bool(v))
        end),
        toggle("r_explicit", tr("f_explicit"), f.explicit, function(v)
          setFilter("explicit", bool(v))
        end),
      }),
    }
    if state.provider == "konachan" and d.konachan_site == "net" then
      table.insert(body, hint(tr("f_konachan_net")))
    end
    table.insert(body, section(tr("f_size")))
    for _, node in ipairs(minSize(f, o)) do
      table.insert(body, node)
    end
    return body
  end

  local function filtersView()
    local d = state.filters
    local title = string.gsub(tr("filters_title"), "{provider}", function()
      return state.providerLabel
    end)
    local body = {
      header(title, {
        ui.button({ text = tr("reset"), variant = "outline", enabled = d ~= nil and not state.busy, onClick = resetFilters }),
      }),
    }
    if state.error ~= "" then
      table.insert(body, hint(state.error, "error"))
    end
    if d == nil then
      table.insert(body, hint(if state.loading then tr("loading") else ""))
    else
      local rest = if state.provider == "wallhaven" then wallhavenFilters(d) else moebooruFilters(d)
      for _, node in ipairs(rest) do
        table.insert(body, node)
      end
      table.insert(body, hint(tr("f_sort_hint")))
    end
    return ui.column({ gap = 12, align = "stretch" }, body)
  end

  -- ---------------------------------------------------------- settings ----

  local function foldersList(s)
    local rows = {}
    for i, folder in ipairs(s.folders or {}) do
      table.insert(rows, ui.row({ key = "folder_" .. folder, gap = 8, align = "center" }, {
        ui.label({ text = folder, color = "on_surface", flexGrow = 1, maxLines = 1 }),
        ui.button({
          glyph = "close",
          variant = "ghost",
          controlSize = "sm",
          tooltip = tr("s_remove_folder"),
          -- The library needs at least one folder.
          enabled = not state.busy and #s.folders > 1,
          onClick = function()
            local keep = {}
            for j, other in ipairs(s.folders) do
              if j ~= i then
                table.insert(keep, other)
              end
            end
            setSetting("folders", table.concat(keep, "\n"))
          end,
        }),
      }))
    end
    table.insert(rows, ui.input({
      key = `add_folder_{state.inputRev}`,
      placeholder = tr("s_add_folder"),
      enabled = not state.busy,
      onSubmit = function(v)
        v = noctalia.string.trim(v)
        if v ~= "" then
          local all = table.clone(s.folders or {})
          table.insert(all, v)
          setSetting("folders", table.concat(all, "\n"))
        end
      end,
    }))
    return ui.column({ gap = 6, align = "stretch" }, rows)
  end

  local function settingsView()
    local s = state.settings
    local body = { header(tr("settings_title")) }
    if state.error ~= "" then
      table.insert(body, hint(state.error, "error"))
    end
    if s == nil then
      table.insert(body, hint(if state.loading then tr("loading") else ""))
      return ui.column({ gap = 12, align = "stretch" }, body)
    end
    local o = s.options

    table.insert(body, section(tr("s_wallpaper")))
    table.insert(body, choice("backend", tr("s_backend"), o.backends, s.backend, function(v)
      setSetting("backend", v)
    end))
    if s.backend == "noctalia" then
      table.insert(body, hint(tr("s_noctalia_hint")))
      if s.noctalia_wallpaper_enabled == false then
        table.insert(body, hint(tr("s_noctalia_off"), "error"))
      end
    else
      table.insert(body, choice("fit_mode", tr("s_fit"), o.fit_modes, s.fit_mode, function(v)
        setSetting("fit_mode", v)
      end))
      -- Each connected monitor's own fit, over the one above.
      local perMonitor = { { value = "", label = tr("s_fit_same") } }
      for _, c in ipairs(o.fit_modes or {}) do
        if c.value ~= "" then
          table.insert(perMonitor, c)
        end
      end
      for _, m in ipairs(deps.monitors() or {}) do
        local current = (s.monitor_fits or {})[m.name] or ""
        table.insert(body, choice("monitor_fit_" .. m.name, `{m.name}  ({m.width}×{m.height})`, perMonitor, current, function(v)
          setSetting("monitor_fit." .. m.name, v)
        end))
      end
      table.insert(body, hint(tr("s_fit_hint")))
    end

    table.insert(body, section(tr("s_library")))
    table.insert(body, hint(tr("s_folders")))
    table.insert(body, foldersList(s))
    table.insert(body, toggle("recursive", tr("s_recursive"), s.recursive, function(v)
      setSetting("recursive", bool(v))
    end))
    table.insert(body, field("thumb_width", tr("s_thumb_width"), tostring(s.thumb_width), "360", 120, function(v)
      setSetting("thumb_width", v)
    end))

    table.insert(body, section(tr("s_downloads")))
    table.insert(body, field("download_dir", tr("s_download_dir"), s.download_dir, "~/Pictures/Wallpapers/wallhaven", nil, function(v)
      setSetting("download_dir", v)
    end))
    table.insert(body, hint(tr("s_enter_saves")))

    for _, node in ipairs(self.slideshowSection(s)) do
      table.insert(body, node)
    end

    table.insert(body, section("Konachan"))
    table.insert(body, choice("konachan_site", tr("s_site"), o.konachan_sites, s.konachan_site, function(v)
      setSetting("konachan_site", v)
    end))

    table.insert(body, section("Wallhaven"))
    table.insert(body, ui.row({ gap = 8, align = "center" }, {
      ui.label({
        text = if s.api_key_set then tr("s_key_set") else tr("s_key_unset"),
        color = if s.api_key_set then "primary" else "on_surface_variant",
        flexGrow = 1,
      }),
      ui.button({ text = tr("open_app"), variant = "outline", onClick = deps.openApp }),
    }))
    table.insert(body, hint(tr("s_key_hint")))
    return ui.column({ gap = 12, align = "stretch" }, body)
  end

  -- Settings → Slideshow: on/off, how often, what from, where.
  local SLIDESHOW_MINUTES = { 5, 10, 15, 30, 60, 120, 240, 480, 1440 }
  function self.slideshowSection(s)
    local show = s.slideshow or {}
    local every = tonumber(show.minutes) or 30
    local function set(key, value)
      setSetting("slideshow." .. key, value)
    end
    local body = { section(tr("s_slideshow")) }
    table.insert(body, toggle("show_on", tr("s_show_on"), show.enabled, function(v)
      set("enabled", bool(v))
    end))
    local minutes = {}
    for _, m in ipairs(SLIDESHOW_MINUTES) do
      local label = if m < 60 then tr("minutes", { n = m }) elseif m == 60 then tr("one_hour") else tr("hours", { n = m // 60 })
      table.insert(minutes, { value = tostring(m), label = label })
    end
    if not table.find(SLIDESHOW_MINUTES, every) then
      table.insert(minutes, 1, { value = tostring(every), label = tr("minutes", { n = every }) })
    end
    table.insert(body, choice("show_minutes", tr("s_show_every"), minutes, tostring(every), function(v)
      set("minutes", v)
    end))
    table.insert(body, toggle("show_favorites", tr("s_show_favorites"), show.favorites, function(v)
      set("favorites", bool(v))
    end))
    -- Tags: chips of the library's tags; All/Any as for the grid.
    local tagItems = {}
    for _, t in ipairs(deps.tags() or {}) do
      table.insert(tagItems, { value = t.name, label = t.name })
    end
    if #tagItems > 0 then
      table.insert(body, hint(tr("s_show_tags")))
      table.insert(body, chips("show_tag_", tagItems, show.tags, function(list)
        set("tags", list)
      end))
      if #(show.tags or {}) > 1 then
        table.insert(body, toggle("show_any", tr("s_show_any"), show.any, function(v)
          set("any", bool(v))
        end))
      end
    end
    table.insert(body, field("show_folder", tr("s_show_folder"), show.folder or "", tr("s_show_folder_any"), nil, function(v)
      set("folder", v)
    end))
    if show.folder and show.folder ~= "" then
      table.insert(body, ui.button({ text = tr("s_show_folder_clear"), variant = "ghost", controlSize = "sm", onClick = function()
        set("folder", "")
      end }))
    end
    -- Monitors: none picked means all of them.
    local monitorItems = {}
    for _, m in ipairs(deps.monitors() or {}) do
      table.insert(monitorItems, { value = m.name, label = m.name })
    end
    if #monitorItems > 1 then
      table.insert(body, hint(tr("s_show_monitors")))
      table.insert(body, chips("show_mon_", monitorItems, show.monitors, function(list)
        set("monitors", list)
      end))
      table.insert(body, toggle("show_each", tr("s_show_each"), show.each, function(v)
        set("each", bool(v))
      end))
    end
    local st = state.slideshowStatus
    local status = if st and st.enabled and st.next_in ~= nil
      then tr("s_show_next", { n = math.max(1, math.ceil(st.next_in / 60)) })
      else tr("s_show_off_hint")
    table.insert(body, ui.row({ gap = 8, align = "center" }, {
      ui.label({ text = status, fontSize = 12, color = "on_surface_variant", flexGrow = 1 }),
      ui.button({ text = tr("s_show_now"), glyph = "arrows-shuffle", variant = "outline", enabled = not state.busy, onClick = changeNow }),
    }))
    return body
  end

  function self.view()
    if state.view == "filters" then
      return filtersView()
    end
    return settingsView()
  end

  return self
end

return M
