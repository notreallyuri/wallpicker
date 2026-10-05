--!nonstrict
-- Finding the wallpicker command: the plugin's "command" setting, else
-- `wallpicker` on Noctalia's PATH, else the usual user bin folders. Hyprland
-- often starts Noctalia with a shorter PATH than your shell's (no
-- ~/.local/bin), so those are looked in too.

local M = {}

M.USER_BIN_DIRS = { "~/.local/bin", "~/.cargo/bin" }

-- The command to run, or nil when it can't be found.
function M.find(noctalia)
  local c = noctalia.getConfig("command")
  c = if type(c) == "string" then noctalia.string.trim(c) else ""
  if c == "" then
    c = "wallpicker"
  end
  -- A path (from the settings) is used as given, if it's there.
  if string.find(c, "/", 1, true) or c:sub(1, 1) == "~" then
    local path = noctalia.expandPath(c)
    return if noctalia.fileExists(path) then path else nil
  end
  if noctalia.commandExists(c) then
    return c
  end
  for _, dir in ipairs(M.USER_BIN_DIRS) do
    local path = noctalia.expandPath(dir .. "/" .. c)
    if noctalia.fileExists(path) then
      return path
    end
  end
  return nil
end

return M
