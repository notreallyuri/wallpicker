--!nonstrict
-- Stand-ins for what Noctalia gives a panel script (ui, panel, noctalia),
-- and canned `wallpicker` output, so the panel runs under plain `luau`.
-- Commands are queued like the real runAsync and answered by flush().

local mock = {}

mock.tree = nil -- the last panel.render() tree
mock.menu = nil -- the last openContextMenu() request
mock.calls = {} -- every command run, as a joined argv
mock.queue = {}
mock.notes = {} -- notify/notifyError/clipboard, in order
mock.closed = 0
mock.files = {} -- writeFile results, by path

-- ------------------------------------------------------------- fixtures ----

local function list()
  return {
    backend = "hyprpaper",
    monitors = {
      { name = "DP-1", width = 2560, height = 1440, focused = true, current = "/w/anime/b.jpg" },
      { name = "DP-2", width = 1920, height = 1080, focused = false, current = "/w/a.jpg" },
    },
    images = {
      { path = "/w/anime/b.jpg", name = "b.jpg", modified = 50, favorite = true, tags = { "Mitsu", "dark" }, thumb = "/c/b.jpg", root = 0, folder = "anime", width = 3840, height = 2160 },
      { path = "/w/a.jpg", name = "a.jpg", modified = 40, favorite = false, tags = { "dark" }, thumb = "/c/a.jpg", root = 0, folder = "", fit = "tile", width = 1920, height = 1080 },
      { path = "/w/anime/mitsu/c.jpg", name = "c.jpg", modified = 30, favorite = false, tags = { "Mitsu" }, thumb = "/c/c.jpg", root = 0, folder = "anime/mitsu" },
      { path = "/w/wallpicker/d.png", name = "d.png", modified = 20, favorite = false, tags = {}, thumb = nil, root = 0, folder = "wallpicker" },
      { path = "/w/e.jpg", name = "e.jpg", modified = 10, favorite = false, tags = {}, thumb = "/c/e.jpg", root = 0, folder = "" },
      { path = "/w/image-5.uhdpaper.com_wallpaper_anime-girl-blonde-short-hair-hd-wallpaper-uhdpaper.com-933@5@l.jpg", name = "image-5.uhdpaper.com_wallpaper_anime-girl-blonde-short-hair-hd-wallpaper-uhdpaper.com-933@5@l.jpg", modified = 5, favorite = false, tags = {}, thumb = "/c/long.jpg", root = 0, folder = "" },
    },
    all_tags = {
      { name = "dark", count = 2 },
      { name = "Mitsu", count = 2 },
      { name = "landscape", count = 1 },
      { name = "night", count = 1 },
      { name = "cityscape", count = 1 },
      { name = "minimalist", count = 1 },
      { name = "abstract", count = 1 },
      { name = "space", count = 1 },
      { name = "pixel art", count = 1 },
      { name = "retrowave", count = 1 },
      { name = "neon lights", count = 1 },
      { name = "rain", count = 1 },
      { name = "ocean", count = 1 },
      { name = "sunset", count = 1 },
    },
    roots = { { path = "/w", name = "w" } },
  }
end

local function result(provider, n, purity, downloaded)
  local id = `{provider}{n}`
  return {
    key = `{provider}:{id}`,
    id = id,
    resolution = "2560x1440",
    purity = purity,
    likes = `♥ {n}`,
    page_url = `https://example.test/{id}`,
    downloaded = downloaded,
    thumb = `/c/online/{id}.jpg`,
    handle = `handle-{id}`,
  }
end

local SORTS = {
  { value = "toplist", label = "Top" },
  { value = "date_added", label = "Latest" },
}

local function search(argv)
  local page = 1
  for i, a in ipairs(argv) do
    if a == "--page" then
      page = tonumber(argv[i + 1]) or 1
    end
  end
  local sort = "date_added"
  for i, a in ipairs(argv) do
    if a == "--sort" then
      sort = argv[i + 1]
    end
  end
  local provider = argv[3]
  return {
    provider = provider,
    providers = {
      { value = "wallhaven", label = "Wallhaven" },
      { value = "konachan", label = "Konachan" },
    },
    sort = sort,
    sorts = SORTS,
    filters_active = 1,
    query = "",
    page = page,
    last_page = 3,
    total = 60,
    items = {
      result(provider, page * 10 + 1, "sfw", false),
      result(provider, page * 10 + 2, "sketchy", true),
      result(provider, page * 10 + 3, "nsfw", false),
    },
  }
end

local function details()
  return {
    provider = "wallhaven",
    provider_label = "Wallhaven",
    title = "2560x1440 · anime",
    stats = { "♥ 12", "300 views", "2.1 MB" },
    page_url = "https://example.test/w1",
    source = "https://example.test/source",
    uploader = { name = "Uploader", query = "@Uploader" },
    artists = { { name = "Mitsu Art", query = "id:107996" } },
    tags = {
      { name = "Mitsu Art", query = "id:107996", category = "Artists", is_artist = true },
      { name = "landscape", query = "id:711", category = "Nature", is_artist = false },
      { name = "night", query = "id:39", category = "Nature", is_artist = false },
    },
    downloaded = false,
    preview = "/c/online/preview/w1.jpg",
  }
end

local function choices(values)
  local out = {}
  for _, v in ipairs(values) do
    table.insert(out, { value = v, label = if v == "" then "Any" else v })
  end
  return out
end

local function filters()
  return {
    wallhaven = { sort = "date_added", range = "1M", atleast = "", resolutions = { "1024x768" }, ratios = { "16x9" }, general = true, anime = true, people = false, sfw = true, sketchy = false, nsfw = true, active = 3 },
    konachan = { sort = "latest", atleast = "", safe = true, questionable = false, explicit = false, active = 0 },
    yandere = { sort = "latest", atleast = "1920x1080", safe = true, questionable = true, explicit = false, active = 2 },
    options = {
      wallhaven_sorts = SORTS,
      moebooru_sorts = choices({ "latest", "score", "random" }),
      top_ranges = choices({ "1d", "1M" }),
      ratios = choices({ "16x9", "4x3" }),
      resolutions = choices({ "1920x1080", "2560x1440" }),
      min_resolutions = choices({ "", "1920x1080" }),
    },
    api_key_set = false,
    konachan_site = "net",
  }
end

local function settings()
  return {
    folders = { "/w", "/x" },
    download_dir = "/w/wallpicker",
    recursive = true,
    backend = "hyprpaper",
    monitor_fits = { ["DP-2"] = "contain" },
    slideshow = { enabled = true, minutes = 15, favorites = false, tags = { "Mitsu" }, any = false, folder = nil, monitors = {}, each = true },
    fit_mode = "",
    thumb_width = 360,
    konachan_site = "net",
    api_key_set = true,
    noctalia_wallpaper_enabled = false,
    options = {
      backends = choices({ "hyprpaper", "noctalia" }),
      fit_modes = choices({ "", "cover", "contain", "tile", "fill" }),
      konachan_sites = choices({ "net", "com" }),
    },
  }
end

-- `wallpicker <command>` -> what it prints (as a table: json.decode below
-- hands it back as is).
local function answer(argv)
  local cmd = argv[2]
  if cmd == "list" then
    return list()
  elseif cmd == "search" then
    return search(argv)
  elseif cmd == "details" then
    return details()
  elseif cmd == "tags" then
    if string.find(argv[4] or "", "mitsu", 1, true) then
      return {
        exact = false,
        suggestions = {
          { name = "mitsu_(mitsu_art)", count = 406, kind = "artist", applied = "night mitsu_(mitsu_art)" },
          { name = "mitsumi_misato", count = 260, kind = "artist", applied = "night mitsumi_misato" },
        },
      }
    end
    return { exact = true, suggestions = {} }
  elseif cmd == "collections" then
    return { { id = 1, label = "Default", count = 16, public = false }, { id = 2, label = "goon", count = 11, public = true } }
  elseif cmd == "collection" then
    local page = 1
    for i, a in ipairs(argv) do
      if a == "--page" then
        page = tonumber(argv[i + 1]) or 1
      end
    end
    local id = tonumber(argv[3]) or 0
    return {
      id = id,
      page = page,
      last_page = 3,
      total = 60,
      items = { result("wallhaven", id * 100 + page, "sfw", false), result("wallhaven", id * 100 + page + 50, "nsfw", true) },
    }
  elseif cmd == "duplicates" and argv[3] == "--json" then
    return {
      {
        likeness = "similar",
        items = {
          { path = "/w/anime/b.jpg", width = 3840, height = 2160, size = 5000000 },
          { path = "/w/a.jpg", width = 1920, height = 1080, size = 900000 },
          { path = "/w/e.jpg", width = 1920, height = 1080, size = 800000 },
        },
      },
    }
  elseif cmd == "slideshow" and argv[3] == "status" then
    return { enabled = true, minutes = 15, next_in = 610 }
  elseif cmd == "preview" then
    return { preview = "/c/online/preview/full.png" }
  elseif cmd == "filters" then
    return filters()
  elseif cmd == "settings" then
    return settings()
  end
  return nil -- favorite, tag, trash, apply...: nothing to print
end

-- ------------------------------------------------------------------ ui ----

local function node(kind)
  return function(props, children)
    return { kind = kind, props = props or {}, children = children or {} }
  end
end

local ui = {}
for _, kind in ipairs({
  "column", "row", "scroll", "box", "label", "markdown", "glyph", "image", "separator", "spacer",
  "progress", "button", "graph", "input", "select", "slider", "toggle",
}) do
  ui[kind] = node(kind)
end
mock.ui = ui

mock.panel = {
  render = function(tree)
    mock.tree = tree
  end,
  close = function()
    mock.closed += 1
  end,
  openContextMenu = function(request)
    mock.menu = request
    return true
  end,
}

-- ------------------------------------------------------------ noctalia ----

local translations = {} -- filled by the runner from translations/en.json

mock.noctalia = {
  tr = function(key, subst)
    local text = translations[key] or `<{key}>`
    for k, v in pairs(subst or {}) do
      text = string.gsub(text, "{" .. k .. "}", function()
        return tostring(v)
      end)
    end
    return text
  end,
  getConfig = function(_key)
    return nil
  end,
  commandExists = function(_name)
    return true
  end,
  fileExists = function(_path)
    return true
  end,
  expandPath = function(path)
    return path
  end,
  string = {
    trim = function(s)
      return (string.gsub(s or "", "^%s*(.-)%s*$", "%1"))
    end,
  },
  json = {
    decode = function(value)
      return value
    end,
    encode = function(_value)
      return "{}"
    end,
  },
  runAsync = function(argv, callback, _timeout)
    table.insert(mock.calls, table.concat(argv, " "))
    if callback then
      table.insert(mock.queue, { argv = argv, callback = callback })
    end
    return true
  end,
  notify = function(title, body)
    table.insert(mock.notes, `notify {title}: {body or ""}`)
  end,
  notifyError = function(title, body)
    table.insert(mock.notes, `error {title}: {body or ""}`)
  end,
  copyToClipboard = function(text, _mime)
    table.insert(mock.notes, `clipboard {text}`)
    return true
  end,
  getenv = function(_name)
    return nil
  end,
  pluginDataDir = function()
    return nil
  end,
  readFile = function(_path)
    return nil
  end,
  writeFile = function(path, contents)
    mock.files[path] = contents
    return true
  end,
}

function mock.setTranslations(t)
  translations = t
end

-- Answer every queued command, as the real runAsync would later.
-- `fail` makes them all fail instead.
function mock.flush(fail)
  while #mock.queue > 0 do
    local job = table.remove(mock.queue, 1)
    if fail then
      job.callback({ exitCode = 1, stdout = "", stderr = "wallpicker: boom", timedOut = false })
    else
      job.callback({ exitCode = 0, stdout = answer(job.argv), stderr = "", timedOut = false })
    end
  end
end

-- ------------------------------------------------------------- finding ----

-- The first node (depth first) for which pred(node) is true.
function mock.find(pred, tree)
  local function walk(n)
    if type(n) ~= "table" or n.kind == nil then
      return nil
    end
    if pred(n) then
      return n
    end
    for _, child in ipairs(n.children) do
      local hit = walk(child)
      if hit then
        return hit
      end
    end
    return nil
  end
  return walk(tree or mock.tree)
end

function mock.byKey(key)
  local n = mock.find(function(n)
    return n.props.key == key
  end)
  assert(n, "no node with key " .. key)
  return n
end

-- A button (or anything clickable) whose text starts with `text`.
function mock.byText(text)
  local n = mock.find(function(n)
    local t = n.props.text
    return type(t) == "string" and string.sub(t, 1, #text) == text and n.props.onClick ~= nil
  end)
  assert(n, "nothing clickable with text " .. text)
  return n
end

-- Call a node's handler: a function, or the name of a global.
function mock.fire(n, prop, ...)
  local h = n.props[prop]
  assert(h ~= nil, `{n.kind} has no {prop}`)
  if type(h) == "string" then
    local f = getfenv()[h] or _G[h]
    assert(f, "no global handler " .. h)
    f(...)
  else
    h(...)
  end
end

-- ----------------------------------------------------------- snapshot ----

-- The tree as stable text: kinds, keys and every non-function prop.
function mock.snapshot(tree)
  local lines = {}
  local function value(v)
    if type(v) == "table" then
      local parts = {}
      for _, x in ipairs(v) do
        table.insert(parts, tostring(x))
      end
      return "[" .. table.concat(parts, "|") .. "]"
    end
    return tostring(v)
  end
  local function walk(n, depth)
    if type(n) ~= "table" or n.kind == nil then
      return
    end
    local names = {}
    for k, v in pairs(n.props) do
      if type(v) ~= "function" then
        table.insert(names, k)
      end
    end
    table.sort(names)
    local parts = { string.rep("  ", depth) .. n.kind }
    for _, k in ipairs(names) do
      table.insert(parts, `{k}={value(n.props[k])}`)
    end
    -- Handlers: present or not (and the global's name), not their code.
    local handlers = {}
    for k, v in pairs(n.props) do
      if type(v) == "function" then
        table.insert(handlers, k)
      end
    end
    table.sort(handlers)
    if #handlers > 0 then
      table.insert(parts, "on:" .. table.concat(handlers, ","))
    end
    table.insert(lines, table.concat(parts, " "))
    for _, child in ipairs(n.children) do
      walk(child, depth + 1)
    end
  end
  walk(tree or mock.tree, 0)
  return table.concat(lines, "\n")
end

return mock
