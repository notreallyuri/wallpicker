--!nonstrict
-- Drives the panel through every view with mock.luau's fake host and canned
-- `wallpicker` output, and prints what each step renders and runs. The
-- runner compares that with smoke.snap, so a refactor that changes what the
-- panel shows or runs shows up as a diff.

local mock = MOCK -- set by run.py before the panel loads
local out = {}

local function step(name)
  mock.flush()
  table.insert(out, "=== " .. name)
  table.insert(out, mock.snapshot())
  if #mock.calls > 0 then
    table.insert(out, "--- ran")
    for _, c in ipairs(mock.calls) do
      table.insert(out, c)
    end
    mock.calls = {}
  end
  if #mock.notes > 0 then
    table.insert(out, "--- notes")
    for _, n in ipairs(mock.notes) do
      table.insert(out, n)
    end
    mock.notes = {}
  end
end

local function menu(label)
  table.insert(out, "--- menu " .. label)
  for _, item in ipairs(mock.menu.items) do
    table.insert(out, `{item.kind or "item"} {item.id or ""} {item.label or ""} {tostring(item.enabled)}`)
  end
  table.insert(out, `context={tostring(mock.menu.context)} onActivate={mock.menu.onActivate}`)
end

-- ---------------------------------------------------------------- local ----

onOpen({})
step("local: opened")

mock.fire(mock.byKey("favorites"), "onClick")
step("local: favorites only")
mock.fire(mock.byKey("favorites"), "onClick")

mock.fire(mock.byKey("filter_0"), "onChange", "mits")
step("local: filter 'mits'")
mock.fire(mock.byKey("filter_0"), "onChange", "")

mock.fire(mock.byKey("tags_expand"), "onClick")
step("local: tags expanded")
mock.fire(mock.byKey("tags_expand"), "onClick")
step("local: tags collapsed again")
mock.fire(mock.byKey("chip_Mitsu"), "onClick")
mock.fire(mock.byKey("chip_dark"), "onClick")
step("local: tags Mitsu + dark (all)")
mock.fire(mock.byKey("tag_mode"), "onClick")
step("local: tags any")
mock.fire(mock.byKey("tags_clear"), "onClick")
mock.fire(mock.byKey("tag_mode"), "onClick")

mock.fire(mock.byKey("local_sort"), "onChange", "2", "Name")
step("local: sort by name")
mock.fire(mock.byKey("local_sort"), "onChange", "0", "Newest")

mock.fire(mock.byKey("now_toggle"), "onClick")
step("local: now showing on")
mock.fire(mock.byKey("now_DP-2").children[1], "onClick")
step("local: DP-2 picked from the strip")
mock.fire(mock.byKey("now_toggle"), "onClick")

mock.fire(mock.byKey("layout"), "onClick")
step("local: folders view")
mock.fire(mock.byKey("folder_anime").children[2], "onClick")
step("local: in anime")
mock.fire(mock.byKey("folder_mitsu").children[2], "onClick")
step("local: in anime/mitsu")
onKey("alt+up", true)
step("local: alt+up")
mock.fire(mock.byKey("crumb_1"), "onClick")
step("local: breadcrumb to top")
mock.fire(mock.byKey("filter_0"), "onChange", "c.jpg")
step("local: searching from the top in folders view")
mock.fire(mock.byKey("filter_0"), "onChange", "")
mock.fire(mock.byKey("layout"), "onClick")

-- The picture applies; the caption opens the wallpaper's view.
mock.fire(mock.byKey("/w/a.jpg").children[1], "onClick")
step("local: apply a.jpg to the target")

mock.fire(mock.byKey("/w/a.jpg").children[2], "onRightClick")
menu("a.jpg")
onLocalMenu("apply_on:DP-1", "/w/a.jpg")
step("local: menu apply to DP-1")
onLocalMenu("apply_all", "/w/a.jpg")
onLocalMenu("copy", "/w/a.jpg")
onLocalMenu("show", "/w/a.jpg")
onLocalMenu("favorite", "/w/a.jpg")
step("local: menu all, copy, show, favorite")

mock.fire(mock.byKey("random"), "onClick")
step("local: random")
mock.fire(mock.byKey("previous"), "onClick")
step("local: previous (on the target monitor)")
onKey("ctrl+r", true)
step("local: ctrl+r")

mock.fire(mock.byKey("/w/anime/b.jpg").children[2], "onClick")
step("local: b.jpg view")
mock.fire(mock.byText("Edit in wallpicker"), "onClick")
step("local: edit tags hands over to the app")
mock.fire(mock.byText("Move to trash"), "onClick")
step("local: trash asks")
mock.fire(mock.byText("Cancel"), "onClick")
onKey("escape", true)
step("local: escape back to the grid")

mock.fire(mock.byKey("duplicates"), "onClick")
step("local: duplicates hand over to the app")
onLocalMenu("tags", "/w/e.jpg")
step("local: menu edit tags hands over to the app")

onLocalMenu("trash", "/w/e.jpg")
step("local: trash from the menu opens the view, asking")
onKey("escape", true)

-- A tag in a wallpaper's view picks it in the grid (and again unpicks it).
mock.fire(mock.byKey("/w/anime/b.jpg").children[2], "onClick")
mock.fire(mock.byKey("tag_Mitsu"), "onClick")
step("local: Mitsu from b.jpg's view, picked in the grid")
mock.fire(mock.byKey("chip_Mitsu"), "onClick")

-- --------------------------------------------------------------- online ----

onKey("ctrl+2", true)
step("online: first search")
onNextPage()
step("online: page 2")
onPrevPage()
step("online: back to page 1 (remembered, nothing ran)")
onKey("alt+right", true)
step("online: alt+right (page 2 remembered)")
onRefresh()
step("online: refresh page 2")

onSortChange("0", "Top")
step("online: sort Top")
onOnlineQueryChange("forest")
onOnlineQuerySubmit("forest")
step("online: search forest")
onProviderChange("1", "Konachan")
step("online: konachan")
onOnlineQueryChange("night mitsu")
step("online: konachan, typing: tag suggestions")
mock.fire(mock.byKey("suggest_mitsu_(mitsu_art)"), "onClick")
step("online: picked the real tag, searched")
onProviderChange("0", "Wallhaven")
mock.flush()

local first = mock.find(function(n)
  return n.kind == "button" and n.props.onRightClick ~= nil
end)
mock.fire(first, "onRightClick")
menu("first online result")
onOnlineMenu("download", mock.menu.context)
step("online: download only")
onOnlineMenu("apply", mock.menu.context)
step("online: apply")

mock.fire(first, "onClick")
step("online: details")
mock.fire(mock.byKey("otag_landscape"), "onClick")
step("online: tag search from details")

mock.fire(mock.byKey("filters"), "onClick")
step("online: filters view")
mock.fire(mock.byKey("pur_sketchy"), "onChange", "true")
step("online: sketchy on")
mock.fire(mock.byKey("res_2560x1440"), "onClick")
step("online: exact 2560x1440 (DP-1) added to 1024x768")
mock.fire(mock.byKey("ratio_4x3"), "onClick")
step("online: ratio 4:3 added")
-- Text fields' keys carry a counter that changes after each save.
local function byKeyPrefix(prefix)
  local n = mock.find(function(n)
    return type(n.props.key) == "string" and string.sub(n.props.key, 1, #prefix) == prefix
  end)
  assert(n, "no node with a key starting " .. prefix)
  return n
end
mock.fire(byKeyPrefix("res_custom_"), "onSubmit", "1366 × 768")
mock.flush()
mock.fire(byKeyPrefix("atleast_custom_"), "onSubmit", "big")
step("online: custom resolution typed, bad minimum refused")
mock.fire(mock.byText("Done"), "onClick")
step("online: filters done, searched again")

-- ------------------------------------------------------ the app's parts ----
-- Collections, Settings and the app itself open in the app.

mock.fire(mock.byKey("mode_collections"), "onClick")
step("collections hand over to the app")
onKey("ctrl+1", true)
mock.fire(mock.find(function(n)
  return n.kind == "button" and n.props.glyph == "settings"
end), "onClick")
step("settings hand over to the app")
onOpenApp()
step("open app: on the tab shown")

-- ---------------------------------------------------------------- errors ----

onRefresh()
mock.flush(true)
step("local: list fails")
onKey("escape", true)
step("escape at the grid closes the panel")
table.insert(out, `closed={mock.closed}`)

print(table.concat(out, "\n"))
