--!nonstrict
-- The slideshow's clock: a Noctalia service that asks wallpicker every
-- minute whether it's time for a new wallpaper (`wallpicker slideshow tick`
-- changes it only when the slideshow is on and due, as set in wallpicker's
-- Settings → Slideshow). Nothing runs while it's off, beyond that check.

local Command = require("./command.luau")

local TICK_MS = 60 * 1000
local running = false

noctalia.setUpdateInterval(TICK_MS)

function update()
  -- One at a time: a slow apply never stacks up.
  if running then
    return
  end
  local cmd = Command.find(noctalia)
  if cmd == nil then
    return
  end
  running = true
  noctalia.runAsync({ cmd, "slideshow", "tick" }, function(_result)
    running = false
  end, TICK_MS)
end
