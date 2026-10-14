--!strict
-- The package's own Luau, held to what Jess holds its own to: the payload (src/) is strict,
-- native, and never `any`; everything else follows the style every file follows.
local style = require '@jess/style'
local test = require '@jess/test'

local suite, expect = test.new(), test.expect

local Payload: style.Options = { directives = { 'strict', 'native' }, noAny = true }

--- The package's folder, from this file's chunk name: `jess package test` runs the suite from
--- whatever folder it was called in.
local function root(): string
  local source = debug.info(1, 's')
  local first = string.sub(source, 1, 1)
  local path = if first == '@' or first == '=' then string.sub(source, 2) else source

  return string.match(path, '^(.*)/tests/[^/]*$') or '.'
end

suite.it('passes @jess/style', function()
  local package = root()
  local payload = `{package}/src/`
  local findings: { string } = {}

  for _, path in style.files { package } do
    local options = if string.sub(path, 1, #payload) == payload then Payload else nil

    for _, finding in style.checkFile(path, options) do
      table.insert(findings, style.format(finding, path))
    end
  end

  expect(table.concat(findings, '\n')).toBe ''
end)

return suite
