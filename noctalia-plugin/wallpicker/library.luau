--!nonstrict
-- The Local tab's logic, without any UI: sorting, tag matching, the folder
-- tree for the nested view, and the panel's remembered preferences.
--
-- Images come from `wallpicker list --json`: { path, name, modified,
-- favorite, tags, artists, thumb, root, folder }, where `root` indexes `roots` (from
-- 0) and `folder` is the folder inside that root ("" at its top, "a/b").

local M = {}

M.SORTS = { "newest", "oldest", "name", "random" }

-- --------------------------------------------------------- preferences ----

local DEFAULT_PREFS = {
  sort = "newest",
  layout = "flat", -- or "nested"
  tagMode = "all", -- or "any"
  showNow = false, -- the "Now showing" strip
  tagsExpanded = false, -- the tag chips: one line, or all of them
}

local function prefsPath(noctalia)
  local dir = noctalia.pluginDataDir()
  return if dir then dir .. "/prefs.json" else nil
end

-- Sort, layout, tag mode and the Now showing strip, as last chosen (survives Noctalia restarts).
function M.loadPrefs(noctalia)
  local prefs = table.clone(DEFAULT_PREFS)
  local path = prefsPath(noctalia)
  local text = path and noctalia.readFile(path)
  local saved = text and noctalia.json.decode(text)
  if type(saved) == "table" then
    if table.find(M.SORTS, saved.sort) then
      prefs.sort = saved.sort
    end
    if saved.layout == "flat" or saved.layout == "nested" then
      prefs.layout = saved.layout
    end
    if saved.tagMode == "all" or saved.tagMode == "any" then
      prefs.tagMode = saved.tagMode
    end
    if type(saved.showNow) == "boolean" then
      prefs.showNow = saved.showNow
    end
    if type(saved.tagsExpanded) == "boolean" then
      prefs.tagsExpanded = saved.tagsExpanded
    end
  end
  return prefs
end

function M.savePrefs(noctalia, prefs)
  local path = prefsPath(noctalia)
  local text = noctalia.json.encode({
    sort = prefs.sort,
    layout = prefs.layout,
    tagMode = prefs.tagMode,
    showNow = prefs.showNow,
    tagsExpanded = prefs.tagsExpanded,
  }, true)
  if path and text then
    noctalia.writeFile(path, text)
  end
end

-- -------------------------------------------------------------- sorting ----

-- Favorites first, then by `mode`. `shuffle` maps path -> number for
-- "random", so the order holds until it's shuffled again.
function M.sorted(images, mode, shuffle)
  local out = table.clone(images)
  local function key(image)
    return shuffle[image.path] or 0
  end
  table.sort(out, function(a, b)
    if (a.favorite == true) ~= (b.favorite == true) then
      return a.favorite == true
    end
    if mode == "oldest" then
      if a.modified ~= b.modified then
        return (a.modified or 0) < (b.modified or 0)
      end
    elseif mode == "name" then
      local an, bn = string.lower(a.name or ""), string.lower(b.name or "")
      if an ~= bn then
        return an < bn
      end
    elseif mode == "random" then
      if key(a) ~= key(b) then
        return key(a) < key(b)
      end
    elseif a.modified ~= b.modified then
      return (a.modified or 0) > (b.modified or 0)
    end
    return a.path < b.path
  end)
  return out
end

function M.newShuffle(images)
  local shuffle = {}
  for _, image in ipairs(images) do
    shuffle[image.path] = math.random()
  end
  return shuffle
end

-- ----------------------------------------------------------------- tags ----

-- Every selected tag ("all") or at least one ("any"), ignoring case.
function M.hasTags(image, selected, mode)
  if #selected == 0 then
    return true
  end
  local mine = {}
  for _, t in ipairs(image.tags or {}) do
    mine[string.lower(t)] = true
  end
  for _, t in ipairs(selected) do
    local has = mine[string.lower(t)] == true
    if mode == "any" and has then
      return true
    end
    if mode ~= "any" and not has then
      return false
    end
  end
  return mode ~= "any"
end

-- ------------------------------------------------------------- folders ----

-- A place in the tree: { root = n, folder = "a/b" }, or nil for the list of
-- wallpaper folders (only offered when there are several).
function M.topOf(roots)
  if #roots <= 1 then
    return { root = 0, folder = "" }
  end
  return nil
end

local function startsWith(s, prefix)
  return string.sub(s, 1, #prefix) == prefix
end

-- Is `image` at `loc` or anywhere below it?
function M.inside(image, loc)
  if loc == nil then
    return true
  end
  if (image.root or 0) ~= loc.root then
    return false
  end
  local f = image.folder or ""
  return loc.folder == "" or f == loc.folder or startsWith(f, loc.folder .. "/")
end

-- What `loc` shows in the nested view: its subfolders, as
-- { name, loc, count, cover } (cover: the first image below it, in the
-- given order), and the images right in it. `images` is already sorted.
function M.contents(images, roots, loc)
  local folders, direct = {}, {}
  local byName = {}
  local function add(name, at, image)
    local entry = byName[name]
    if entry == nil then
      entry = { name = name, loc = at, count = 0, cover = image }
      byName[name] = entry
      table.insert(folders, entry)
    end
    entry.count += 1
  end
  for _, image in ipairs(images) do
    local root = image.root or 0
    local f = image.folder or ""
    if loc == nil then
      local r = roots[root + 1]
      add(if r then r.name else "?", { root = root, folder = "" }, image)
    elseif root == loc.root then
      if f == loc.folder then
        table.insert(direct, image)
      elseif loc.folder == "" or startsWith(f, loc.folder .. "/") then
        local rest = if loc.folder == "" then f else string.sub(f, #loc.folder + 2)
        local name = string.match(rest, "^[^/]+")
        if name then
          local sub = if loc.folder == "" then name else loc.folder .. "/" .. name
          add(name, { root = root, folder = sub }, image)
        end
      end
    end
  end
  if loc ~= nil then
    table.sort(folders, function(a, b)
      return string.lower(a.name) < string.lower(b.name)
    end)
  end
  return folders, direct
end

-- The way from the top to `loc`, as { label, loc } steps for a breadcrumb.
function M.trail(roots, loc, allLabel)
  local steps = {}
  if #roots > 1 then
    table.insert(steps, { label = allLabel, loc = nil })
  end
  if loc == nil then
    return steps
  end
  local r = roots[loc.root + 1]
  table.insert(steps, { label = if r then r.name else "?", loc = { root = loc.root, folder = "" } })
  local path = ""
  for part in string.gmatch(loc.folder, "[^/]+") do
    path = if path == "" then part else path .. "/" .. part
    table.insert(steps, { label = part, loc = { root = loc.root, folder = path } })
  end
  return steps
end

-- One level up from `loc`, or nil when already at the top.
function M.parent(roots, loc)
  if loc == nil then
    return nil
  end
  if loc.folder == "" then
    if #roots > 1 then
      return { up = true, loc = nil }
    end
    return nil
  end
  local up = string.match(loc.folder, "^(.*)/[^/]+$") or ""
  return { up = true, loc = { root = loc.root, folder = up } }
end

-- The folder `loc` stands for, as a path (for `xdg-open`), or nil.
function M.pathOf(roots, loc)
  if loc == nil then
    return nil
  end
  local r = roots[loc.root + 1]
  if r == nil then
    return nil
  end
  return if loc.folder == "" then r.path else r.path .. "/" .. loc.folder
end

-- Is `loc` still a place the library has (after a reload, say)?
function M.exists(images, roots, loc)
  if loc == nil then
    return #roots > 1
  end
  if roots[loc.root + 1] == nil then
    return false
  end
  for _, image in ipairs(images) do
    if M.inside(image, loc) then
      return true
    end
  end
  return loc.folder == ""
end

return M
