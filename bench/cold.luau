--!strict
-- What one run of the command pays before anything is warm: this package's two modules compiled
-- and run, with the modules they stand on loaded first and timed apart, then the first run of the
-- command line over a small install (a master of 60 lights and 20 cells and two plugins of 12
-- lights), once for the lights and once more with the cells. Run by `jess run --capability
-- filesystem.read --capability filesystem.write --capability process.environment --package .
-- --package-path .. bench/cold.luau <scratch>`, on a release build. Each line is one sample of
-- something a process does once, so compare medians over many runs. This file uses none of the
-- l3i dialect, so that whatever the modules' first use of it costs is paid inside the `require`
-- timed here.
local started = os.clock()
local bytes = require '@dream/bytes'
local fs = require '@dream/fs'
require '@dream/process'
local tes3 = require '@tes3'
require '@load-order'
require '@openmw'
local requiredBelow = os.clock() - started

if not jess.bench.verified then
  error(
    's3lightfixes bench: this jess was built without the verified toolchain: measure on a release build',
    0
  )
end

local scratch: string? = ...
local given: string = assert(scratch, 's3lightfixes bench: the first argument is a scratch folder')

local function made<Value>(value: Value?, message: string?, kind: dream_fs_ErrorKind?): Value
  if not value then error(message or tostring(kind), 0) end

  return value
end

local PluginName = 'S3LightFixes.omwaddon'
local Flags = { 'DYNAMIC', 'DYNAMIC | CAN_CARRY | FIRE | FLICKER_SLOW', 'CAN_CARRY | PULSE', '' }

local function light(id: string, index: number): { [string]: unknown }
  return {
    type = 'Light',
    id = id,
    mesh = `l\\{id}.nif`,
    data = {
      weight = 1,
      value = 5,
      time = if index % 3 == 0 then -1 else index * 40,
      radius = 64 + index * 7,
      color = { 255 - index, (index * 37) % 256, (index * 91) % 256, 0 },
      flags = Flags[index % #Flags + 1],
    },
  }
end

local function cell(index: number): { [string]: unknown }
  return {
    type = 'Cell',
    name = `Cold Cellar {index}`,
    data = { flags = 'IS_INTERIOR', grid = { 0, 0 } },
    atmosphereData = {
      ambientColor = { index, 20, 30, 0 },
      sunlightColor = { 40, index * 3, 60, 0 },
      fogColor = { 70, 80, index * 5, 0 },
      fogDensity = 0.5,
    },
  }
end

local root = jess.fs.join(made(fs.canonicalize(made(fs.absolute(given)))), 's3lightfixes-cold')
local data, config, out =
  jess.fs.join(root, 'data'), jess.fs.join(root, 'config'), jess.fs.join(root, 'out')
local configFile = jess.fs.join(config, 'openmw.cfg')

if jess.fs.isDir(root) then made(fs.remove(root, { recursive = true })) end

for _, directory in { data, config, out } do
  made(fs.mkdir(directory, { recursive = true }))
end

local function save(plugin: tes3.Plugin, name: string)
  plugin:updateRecordCount()
  jess.fs.write(jess.fs.join(data, name), plugin:save())
end

local base = tes3.new 'master'
base:setAuthor 'bench'

for index = 1, 60 do
  base:insertValue(base:count(), light(`cold_light_{index}`, index))
end

for index = 1, 20 do
  base:insertValue(base:count(), cell(index))
end

save(base, 'Base.esm')

for number = 1, 2 do
  local plugin = tes3.new 'plugin'
  plugin:setAuthor 'bench'
  plugin:setMasters { { name = 'Base.esm', size = 0 } }

  for index = 1, 12 do
    local id = if index % 2 == 0 then `COLD_LIGHT_{index * number}` else `cold_mod{number}_{index}`
    plugin:insertValue(plugin:count(), light(id, index + number))
  end

  save(plugin, `Mod_{number}.esp`)
end

jess.fs.write(
  configFile,
  `data="{data}"\ndata-local="{out}"\ncontent=Base.esm\ncontent=Mod_1.esp\ncontent=Mod_2.esp\n`
)

started = os.clock()
local cli = require '@s3lightfixes/cli'
local required = os.clock() - started

started = os.clock()
local first = cli.run { '--config', configFile, '-n' }
local ran = os.clock() - started
local lights = bytes.sha256(made(jess.fs.readBuffer(jess.fs.join(out, PluginName))))

started = os.clock()
local second = cli.run { '--config', configFile, '-n', '--classic' }
local relit = os.clock() - started
local cells = bytes.sha256(made(jess.fs.readBuffer(jess.fs.join(out, PluginName))))

made(fs.remove(root, { recursive = true }))

if first ~= cli.Exit.ok or second ~= cli.Exit.ok then
  error(`s3lightfixes bench: the cold runs exited {first} and {second}`, 0)
end

-- What the two plugins came to before anything here was changed.
local Lights = '95c725fdd393ea12f91f9d28d08c90ba135e41e2eda304619d994c03bdb4d1bd'
local Cells = '3b440c6811af3ac90efee7e21316f923056a9f9b1cd8cdea1c3761f3b3faeaf5'

if lights ~= Lights or cells ~= Cells then
  error(`s3lightfixes bench: the cold plugins came to {lights} and {cells}`, 0)
end

local function row(name: string, seconds: number) print(`{name}\t{string.format('%.9f', seconds)}`) end

row('cold: require the modules below this package', requiredBelow)
row('cold: require @s3lightfixes/cli, those loaded', required)
row('cold: first run, the lights of 3 plugins', ran)
row('cold: second run, lights and cells', relit)
row('cold: require @s3lightfixes/cli to the end of the first run', required + ran)
