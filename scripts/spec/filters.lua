-- Pandoc Lua filters for the one-page HTML build of docs/spec/.

-- Turn priority markers (P0, P1, P2, P0/P1) into colored badges in HTML output.
function Str(el)
  if el.text:match("^P[0-2]$") or el.text:match("^P[0-2]/P[0-2]$") then
    local out = el.text:gsub("P([0-2])", '<span class="prio p%1">P%1</span>')
    return pandoc.RawInline("html", out)
  end
end

-- The parts are concatenated into one page, so cross-part links become in-page links:
--   02-features.md#cap-11 -> #cap-11, README.md#priorities -> #priorities,
--   02-features.md -> #02-features (id of the part's first heading), README.md -> #.
function Link(el)
  local path, frag = el.target:match("^([^#]*)#?(.*)$")
  path = path:gsub("^%./", "")
  local stem = path:match("^(%d%d%-[%w%-]+)%.md$")
  if stem or path == "README.md" then
    if frag ~= "" then
      el.target = "#" .. frag
    elseif stem then
      el.target = "#" .. stem
    else
      el.target = "#"
    end
    return el
  end
end

-- Drop pandoc's relative column widths so the browser sizes columns to content.
function Table(tbl)
  for i, spec in ipairs(tbl.colspecs) do
    tbl.colspecs[i] = { spec[1], pandoc.ColWidthDefault }
  end
  return tbl
end

-- A paragraph holding only `<a id="x"></a>` right before a heading: give the heading
-- that id (so the TOC links to it) and drop the paragraph (no duplicate ids).
local function anchor_id(blk)
  if blk.t ~= "Para" and blk.t ~= "Plain" then return nil end
  local html = ""
  for _, il in ipairs(blk.content) do
    if il.t == "RawInline" and il.format == "html" then
      html = html .. il.text
    elseif il.t ~= "Space" and il.t ~= "SoftBreak" then
      return nil
    end
  end
  return html:match('^<a id="([^"]+)"></a>$')
end

function Blocks(blocks)
  local out = pandoc.Blocks({})
  local i = 1
  while i <= #blocks do
    local id = anchor_id(blocks[i])
    local nxt = blocks[i + 1]
    if id and nxt and nxt.t == "Header" then
      nxt.identifier = id
      out:insert(nxt)
      i = i + 2
    else
      out:insert(blocks[i])
      i = i + 1
    end
  end
  return out
end
