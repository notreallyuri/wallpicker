--!nonstrict
-- Bar widget that opens the wallpicker panel.

local function render()
  barWidget.setGlyph(noctalia.getConfig("glyph") or "photo")
  barWidget.setTooltip(noctalia.tr("title"))
end

function update()
  render()
end

function onClick()
  noctalia.togglePanel("wallpicker/wallpicker:picker")
end

render()
