--!nonstrict
-- The panel's Filters view (per provider). It reads and saves through the
-- wallpicker command, so it's the same file as the app's Filters dialog:
--   wallpicker filters --json                          -> every provider's filters
--   wallpicker filters <provider> set KEY=VALUE | reset
-- Every change is saved as it's made, like in the app. Settings are the
-- app's alone (the panel's gear opens them there).

local M = {}

local LABEL_WIDTH = 170
local SELECT_WIDTH = 280
local TIMEOUT_MS = 30000

-- deps: ui, noctalia, tr, command, render, width, flow (wrapping chips),
-- monitors() (for their sizes), and onClosed(changed), where changed has
-- `filters` set when the results need searching again.
function M.new(deps)
  local ui, noctalia, tr = deps.ui, deps.noctalia, deps.tr

  local state = {
    view = nil, -- "filters" while open
    provider = "wallhaven",
    providerLabel = "Wallhaven",
    filters = nil, -- from `wallpicker filters`
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
      local rest = if state.provider == "wallhaven"
        then wallhavenFilters(d)
        elseif state.provider == "unsplash" then { hint(tr("f_unsplash")) }
        else moebooruFilters(d)
      for _, node in ipairs(rest) do
        table.insert(body, node)
      end
      table.insert(body, hint(tr("f_sort_hint")))
    end
    return ui.column({ gap = 12, align = "stretch" }, body)
  end

  function self.view()
    return filtersView()
  end

  return self
end

return M
