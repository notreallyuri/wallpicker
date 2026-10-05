--!nonstrict
-- Noctalia's corner rounding, so the panel's thumbnails round like its own
-- buttons. The plugin API doesn't expose it, so it's read from Noctalia's
-- settings file: `corner_radius_scale` under `[shell]` (1 when unset or
-- unreadable; 0 is square corners).

local M = {}

-- The scale in a settings.toml's text.
function M.parseCornerScale(text)
  if type(text) ~= "string" then
    return 1
  end
  local section = nil
  for line in string.gmatch(text, "[^\n]+") do
    local l = string.match(line, "^%s*(.-)%s*$")
    local header = string.match(l, "^%[([^%[%]]+)%]$")
    if header then
      section = header
    elseif string.sub(l, 1, 1) == "[" then
      section = nil -- an array of tables ([[...]]): not [shell]
    elseif section == "shell" then
      local v = string.match(l, "^corner_radius_scale%s*=%s*([%d%.]+)")
      if v then
        local n = tonumber(v)
        return if n and n >= 0 then n else 1
      end
    end
  end
  return 1
end

function M.cornerScale(noctalia)
  local base = noctalia.getenv("XDG_STATE_HOME")
  if base == nil or base == "" then
    base = "~/.local/state"
  end
  local text = noctalia.readFile(base .. "/noctalia/settings.toml")
  return M.parseCornerScale(text)
end

return M
