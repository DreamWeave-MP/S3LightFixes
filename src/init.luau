--!strict
--!native
--[[
@s3lightfixes: S3LightFixes 0.4.59, as a function of (state, openmw), openmw being the load
order's config from `@openmw`'s `new`.

Settings come from lightconfig.toml next to the user's openmw.cfg, as they do for the original, with
`state.s3lightfixes` laid over them:
  disableFlickering, disablePulse, disableNegativeLights,
  autoEnable, noNotifications, debug, dryRun, validateConfig, updateLightConfig    boolean
  disableInteriorSun                boolean, or { string } of cell id patterns to disable it in
  standardHue, standardSaturation, standardValue, standardRadius,
  coloredHue, coloredSaturation, coloredValue, coloredRadius, durationMult          number
  output                            string; the folder to write the plugin to
  excludedIds, excludedPlugins      { string }; added to the config's, each once
  lightOverrides                    { { id = pattern, radius = 255, flag = "FLICKER_SLOW", ... } }
  ambientOverrides                  { { id = pattern, fog = { red, green, blue }, fogDensity, ... } }
Overrides take lightconfig.toml's fields, camelCased, and apply after the file's.

Like the original, it writes lightconfig.toml when there's none (or when saveConfig or
updateLightConfig ask), deletes the old S3LightFixes.omwaddon from data-local and writes the new
one there (or to `output`), and writes lightconfig.log beside openmw.cfg. It leaves openmw.cfg
alone unless autoEnable is set, when it backs it up and enables the plugin if it's in a data folder
(the original also enables one OpenMW can't find). Its plugin, lightconfig.toml, and lightconfig.log are the
original's byte for byte, except that overrides apply in the order lightconfig.toml lists them
(the original sorts them by pattern), an override's `flag` never adds or removes CAN_CARRY,
disable_interior_sun can list the cells to disable it in, override patterns ignore case, saved
exclusions are never listed twice, and plugins are read in openmw.cfg's encoding (the original
always reads Windows-1252). Afterwards `state.s3lightfixes.report` holds
{ output, masters, objects, lights, cells, enabled, logs }.

Ported from S3LightFixes (https://github.com/DreamWeave-MP/S3LightFixes):

MIT License

Copyright (c) 2024-2026 Dave Corley (S3kshun8), with contributions from arrean (McArren)

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
]]

local loadOrder = require '@load-order'
local openmwPackage = require '@openmw'
local tes3 = require '@tes3'

type Context = openmwPackage.Context

type Hsv = { hue: number, saturation: number, value: number }

type LightOverride = {
  color: { number }?,
  redMult: number?,
  greenMult: number?,
  blueMult: number?,
  hue: number?,
  saturation: number?,
  value: number?,
  hueMult: number?,
  saturationMult: number?,
  valueMult: number?,
  radius: number?,
  radiusMult: number?,
  duration: number?,
  durationMult: number?,
  flags: number?,
}

type Rgb = { red: number, green: number, blue: number }

type AmbientOverride = { ambient: Rgb?, sunlight: Rgb?, fog: Rgb?, fogDensity: number? }

type OrderedMap<T> = { keys: { string }, values: { [string]: T } }

-- lightconfig.toml, keyed as the file spells it.
type LightConfig = {
  bools: { [string]: boolean },
  floats: { [string]: number },
  excludedPlugins: { string },
  excludedIds: { string },
  -- disable_interior_sun given as a list of cell id patterns.
  interiorSunCells: { string }?,
  lightOverrides: OrderedMap<LightOverride>,
  ambientOverrides: OrderedMap<AmbientOverride>,
  outputDir: string?,
}

type Matcher<T> = { regex: Regex, value: T }

type LightValues = { time: number, radius: number, color: { number }, flags: number }

type RecordLog = { kind: string, plugin: string, id: string, changes: { string } }

-- A LIGH and a CELL as `@tes3` decodes them: the fields this reads, and writes back.
type Light = {
  type: 'Light',
  flags: string,
  id: string,
  name: string,
  script: string,
  mesh: string,
  icon: string,
  sound: string,
  data: {
    weight: number,
    value: number,
    time: number,
    radius: number,
    color: { number },
    flags: string,
  },
}

type AtmosphereData = {
  ambientColor: { number },
  sunlightColor: { number },
  fogColor: { number },
  fogDensity: number,
}

type Cell = {
  type: 'Cell',
  flags: string,
  name: string,
  data: { flags: string, grid: { number } },
  region: string?,
  mapColor: { number }?,
  atmosphereData: AtmosphereData?,
  references: { [unknown]: unknown },
}

type Master = { name: string, size: number }

export type Report = {
  output: string?,
  masters: number?,
  objects: number?,
  lights: number?,
  cells: number?,
  enabled: boolean?,
  logs: { RecordLog }?,
  validated: boolean?,
  dryRun: boolean?,
}

export type LightOverrideOption = {
  id: string,
  red: number?,
  green: number?,
  blue: number?,
  redMult: number?,
  greenMult: number?,
  blueMult: number?,
  hue: number?,
  saturation: number?,
  value: number?,
  hueMult: number?,
  saturationMult: number?,
  valueMult: number?,
  radius: number?,
  radiusMult: number?,
  duration: number?,
  durationMult: number?,
  flag: (string | { string })?,
}

export type RgbOption = {
  red: number?,
  green: number?,
  blue: number?,
  hue: number?,
  saturation: number?,
  value: number?,
}

export type AmbientOverrideOption = {
  id: string,
  ambient: RgbOption?,
  sunlight: RgbOption?,
  fog: RgbOption?,
  fogDensity: number?,
}

export type Options = {
  disableInteriorSun: (boolean | { string })?,
  autoEnable: boolean?,
  noNotifications: boolean?,
  debug: boolean?,
  dryRun: boolean?,
  validateConfig: boolean?,
  updateLightConfig: boolean?,
  disableFlickering: boolean?,
  disablePulse: boolean?,
  disableNegativeLights: boolean?,
  standardHue: number?,
  standardSaturation: number?,
  standardValue: number?,
  standardRadius: number?,
  coloredHue: number?,
  coloredSaturation: number?,
  coloredValue: number?,
  coloredRadius: number?,
  durationMult: number?,
  output: string?,
  excludedIds: { string }?,
  excludedPlugins: { string }?,
  lightOverrides: { LightOverrideOption }?,
  ambientOverrides: { AmbientOverrideOption }?,
  report: Report?,
}

local Version, PluginName = require '@self/version', 'S3LightFixes.omwaddon'
local ConfigName, LogName = 'lightconfig.toml', 'lightconfig.log'
local U8Max, U32Max, I32Min, I32Max = 255, 4294967295, -2147483648, 2147483647

-- An error's message, without the script position `error` puts in front of it.
local function problem(caught: unknown): string
  return (string.gsub(tostring(caught), '^[^\n]-:%d+: ', ''))
end

-- S3LightFixes, and the `palette` crate it converts colors with, compute in f32. Rounding each
-- operation's exact result to f32 reproduces them bit for bit.
local scratch = buffer.create(4)

local function f32(value: number): number
  buffer.writef32(scratch, 0, value)
  return buffer.readf32(scratch, 0)
end

local function isNan(value: number): boolean return value ~= value end

local function isFinite(value: number): boolean
  return not isNan(value) and math.abs(value) ~= math.huge
end

-- f32::max and f32::min ignore a NaN operand; f32::clamp keeps it.
local function fmax(left: number, right: number): number
  if isNan(left) then return right end
  if isNan(right) then return left end

  return if left > right then left else right
end

local function fmin(left: number, right: number): number
  if isNan(left) then return right end
  if isNan(right) then return left end

  return if left < right then left else right
end

local function fclamp(value: number, low: number, high: number): number
  if value < low then return low end
  if value > high then return high end

  return value
end

-- Rust's float-to-integer `as`: saturating, truncating, and NaN becomes 0.
local function toInteger(value: number, low: number, high: number): number
  if isNan(value) then return 0 end
  if value <= low then return low end
  if value >= high then return high end

  return if value < 0 then math.ceil(value) else math.floor(value)
end

-- palette multiplies by the f32 reciprocal of 255 rather than dividing by 255.
local Reciprocal255 = f32(1 / 255)

local function byteToUnit(value: number): number return f32(value * Reciprocal255) end

-- palette rounds half to even here.
local function unitToByte(value: number): number
  local scaled = fmin(f32(value * 255), 255)
  if scaled <= 0 then return 0 end

  local rounded = math.floor(scaled + 0.5)
  if rounded - scaled == 0.5 and rounded % 2 == 1 then
    rounded -= 1
  end

  return rounded
end

local function positiveDegrees(hue: number): number
  return f32(hue - f32(math.floor(f32(hue / 360)) * 360))
end

local function rgbToHsv(red: number, green: number, blue: number): Hsv
  red, green, blue = fmax(red, 0), fmax(green, 0), fmax(blue, 0)

  local max, min, separation, offset
  if red > green then
    max, min, separation, offset = red, green, f32(green - blue), 0
  else
    max, min, separation, offset = green, red, f32(blue - red), 2
  end

  if blue > max then
    max, separation, offset = blue, f32(red - green), 4
  elseif blue < min then
    min = blue
  end

  local hue, saturation = 0, 0
  if max ~= min then
    local delta = f32(max - min)
    hue = f32(f32(f32(separation / delta) + offset) * 60)
    saturation = f32(delta / max)
  end

  return { hue = hue, saturation = fclamp(saturation, 0, 1), value = fclamp(max, 0, 1) }
end

local function hsvToRgb(hsv: Hsv): (number, number, number)
  local chroma = f32(hsv.value * hsv.saturation)
  local sector = f32(positiveDegrees(hsv.hue) / 60)
  local sectorModTwo = f32(sector - f32(math.floor(f32(sector * 0.5)) * 2))
  local secondary = f32(chroma * f32(1 - math.abs(f32(sectorModTwo - 1))))
  local lift = f32(hsv.value - chroma)

  local zone0, zone1 = sector >= 0 and sector < 1, sector >= 1 and sector < 2
  local zone2, zone3 = sector >= 2 and sector < 3, sector >= 3 and sector < 4
  local zone4 = sector >= 4 and sector < 5

  local red = if zone1 or zone4 then secondary elseif zone2 or zone3 then 0 else chroma
  local green = if zone0 or zone3 then secondary elseif zone1 or zone2 then chroma else 0
  local blue = if zone0 or zone1 then 0 elseif zone3 or zone4 then chroma else secondary

  return fclamp(f32(red + lift), 0, 1),
    fclamp(f32(green + lift), 0, 1),
    fclamp(f32(blue + lift), 0, 1)
end

local function colorToHsv(color: { number }): Hsv
  return rgbToHsv(byteToUnit(color[1]), byteToUnit(color[2]), byteToUnit(color[3]))
end

local function hsvToColor(hsv: Hsv): { number }
  local red, green, blue = hsvToRgb(hsv)
  return { unitToByte(red), unitToByte(green), unitToByte(blue), 0 }
end

local function trim(text: string): string return (string.gsub(text, '^%s*(.-)%s*$', '%1')) end

local function split(text: string, separator: string): { string }
  local parts, start = {}, 1

  while true do
    local found = string.find(text, separator, start, true)
    if not found then
      table.insert(parts, string.sub(text, start))
      return parts
    end

    table.insert(parts, string.sub(text, start, found - 1))
    start = found + #separator
  end
end

local function bytesLess(left: string, right: string): boolean
  for index = 1, math.min(#left, #right) do
    local leftByte, rightByte = string.byte(left, index), string.byte(right, index)
    if leftByte ~= rightByte then return leftByte < rightByte end
  end

  return #left < #right
end

local FlagNames = {
  'DYNAMIC',
  'CAN_CARRY',
  'NEGATIVE',
  'FLICKER',
  'FIRE',
  'OFF_BY_DEFAULT',
  'FLICKER_SLOW',
  'PULSE',
  'PULSE_SLOW',
}

local FlagBits: { [string]: number } = {}
for index, name in FlagNames do
  FlagBits[name] = bit32.lshift(1, index - 1)
end

local function flagsFromJess(text: string): number
  local bits = 0

  for _, part in split(text, '|') do
    local name = trim(part)
    if name == '' then continue end

    local bit = FlagBits[name] or tonumber(name) or error(`unknown light flag {name}`)
    bits = bit32.bor(bits, bit)
  end

  return bits
end

local function flagNames(bits: number): { string }
  local names = {}
  for _, name in FlagNames do
    if bit32.band(bits, FlagBits[name]) ~= 0 then table.insert(names, name) end
  end

  return names
end

local function flagsToJess(bits: number): string
  local names = flagNames(bits)
  local unknownBits = bit32.band(bits, bit32.bnot(0x1FF))
  if unknownBits ~= 0 then table.insert(names, string.format('0x%x', unknownBits)) end

  return table.concat(names, ' | ')
end

local function flagsDebug(bits: number): string
  return if bits == 0 then 'LightFlags(0x0)' else `LightFlags({flagsToJess(bits)})`
end

-- Rust's f32 Display: the shortest decimal that reads back as the same f32, without an exponent.
local function displayF32(value: number): string
  if isNan(value) then return 'NaN' end
  if value == math.huge then return 'inf' end
  if value == -math.huge then return '-inf' end
  if value == 0 then return if 1 / value < 0 then '-0' else '0' end

  local magnitude = math.abs(value)
  local digits, exponent = '', 0

  for precision = 0, 8 do
    local text = string.format(`%.{precision}e`, magnitude)
    if f32(tonumber(text) :: number) ~= magnitude then continue end

    local mantissa, power = string.match(text, '^([%d%.]+)e([%+%-]%d+)$')
    digits = string.gsub(mantissa :: string, '%.', '')
    exponent = tonumber(power) :: number
    break
  end

  digits = string.gsub(digits, '0+$', '')
  if digits == '' then digits = '0' end

  local sign = if value < 0 then '-' else ''
  local point = exponent + 1
  if point <= 0 then return sign .. '0.' .. string.rep('0', -point) .. digits end
  if point >= #digits then return sign .. digits .. string.rep('0', point - #digits) end

  return sign .. string.sub(digits, 1, point) .. '.' .. string.sub(digits, point + 1)
end

local DebugEscapes: { [string]: string } = {
  ['\\'] = '\\\\',
  ['"'] = '\\"',
  ['\n'] = '\\n',
  ['\r'] = '\\r',
  ['\t'] = '\\t',
  ['\0'] = '\\0',
}

local function debugString(text: string): string
  local escaped = string.gsub(
    text,
    '[%c\\"]',
    function(character: string): string
      return DebugEscapes[character] or string.format('\\u{%x}', string.byte(character))
    end
  )

  return '"' .. escaped .. '"'
end

local function debugArray(values: { number }): string
  return '[' .. table.concat(values, ', ') .. ']'
end

local function integer(value: number): string return string.format('%d', value) end

-- Settings are checked as the original reads them, and a bad one stops the run with a message
-- naming where it came from: lightconfig.toml, or an override in `state.s3lightfixes`.
local ConfigOrigin = 'Lightconfig.toml couldn\'t be read: '

local function invalid(origin: string, message: string): never error(origin .. message) end

local function parseFlagTokens(origin: string, tokens: { string }): number
  if #tokens == 0 then return invalid(origin, 'Unknown light flag: ``') end

  local bits, none, real = 0, false, false

  for _, raw in tokens do
    local token = trim(raw)
    local name = string.upper(token)
    if token == '' or (name ~= 'NONE' and not FlagBits[name]) then
      return invalid(origin, `Unknown light flag: \`{token}\``)
    end

    if name == 'NONE' then
      none = true
    else
      real = true
      bits = bit32.bor(bits, FlagBits[name])
    end

    if none and real then
      return invalid(origin, 'Light flag `NONE` cannot be combined with `other light flags`')
    end
  end

  return bits
end

local function parseFlags(origin: string, text: string): number
  local tokens = split(text, '|')
  for _, token in tokens do
    if trim(token) == '' then return invalid(origin, `Unknown light flag: \`{text}\``) end
  end

  return parseFlagTokens(origin, tokens)
end

local BoolKeys = {
  'disable_interior_sun',
  'disable_flickering',
  'disable_pulse',
  'disable_negative_lights',
  'auto_enable',
  'no_notifications',
  'debug',
  'dry_run',
  'validate_config',
}

local FloatKeys = {
  'standard_hue',
  'standard_saturation',
  'standard_value',
  'standard_radius',
  'colored_hue',
  'colored_saturation',
  'colored_value',
  'colored_radius',
  'duration_mult',
}

local function defaultConfig(): LightConfig
  return {
    bools = {
      disable_interior_sun = false,
      disable_flickering = true,
      disable_pulse = false,
      disable_negative_lights = true,
      auto_enable = false,
      no_notifications = false,
      debug = false,
      dry_run = false,
      validate_config = false,
      save_config = false,
    },

    floats = {
      standard_hue = f32(0.62),
      standard_saturation = f32(0.8),
      standard_value = f32(0.57),
      standard_radius = f32(1.2),
      colored_hue = f32(1.0),
      colored_saturation = f32(0.9),
      colored_value = f32(0.7),
      colored_radius = f32(1.1),
      duration_mult = f32(2.5),
    },

    excludedPlugins = {
      'deleted_groundcover.omwaddon',
      'Clean_Argonian Full Helms Lore Integrated.ESP',
      'Baldurwind.omwaddon',
      'Crassified Navigation.omwaddon',
      'LuaMultiMark.omwaddon',
      'S3maphore.esp',
      'Toolgun.omwaddon',
    },

    excludedIds = {},
    lightOverrides = { keys = {}, values = {} },
    ambientOverrides = { keys = {}, values = {} },
  }
end

-- OrderedHashMap::insert moves a key that's already there to the end.
local function insertOrdered<T>(map: OrderedMap<T>, key: string, value: T)
  local index = table.find(map.keys, key)
  if index then table.remove(map.keys, index) end

  table.insert(map.keys, key)
  map.values[key] = value
end

-- A TOML false is a value, so these tell a missing field from one set to false.
local function tomlBool(origin: string, value: TomlValue, key: string): boolean
  if type(value) == 'boolean' then return value end

  return invalid(origin, `invalid type for \`{key}\`: expected a boolean`)
end

local function tomlF32(origin: string, value: TomlValue?, key: string): number?
  if value == nil then return nil end
  if type(value) == 'number' then return f32(value) end

  return invalid(origin, `invalid type for \`{key}\`: expected f32`)
end

local function tomlUint(
  origin: string,
  value: TomlValue?,
  isFloat: boolean,
  max: number,
  key: string
): number?
  if value == nil then return nil end

  local expected = if max == U8Max then 'u8' else 'u32'
  if type(value) ~= 'number' or isFloat then
    return invalid(origin, `invalid type for \`{key}\`: expected {expected}`)
  end

  if value < 0 or value > max or value % 1 ~= 0 then
    return invalid(origin, `invalid value for \`{key}\`: integer \`{value}\`, expected {expected}`)
  end

  return value
end

local function tomlStrings(origin: string, value: TomlValue, key: string): { string }
  if type(value) ~= 'table' then
    return invalid(origin, `invalid type for \`{key}\`: expected a sequence`)
  end

  local strings = {}
  for _, item in ipairs(value :: { TomlValue }) do
    if type(item) ~= 'string' then
      return invalid(origin, `invalid type for \`{key}\`: expected a string`)
    end

    table.insert(strings, item)
  end

  return strings
end

-- The original reads overrides sorted by pattern (its toml tables are BTreeMaps); these apply in
-- the order the file lists them.
local function keysIn(layout: TomlLayout, value: TomlTable): { string }
  local entry = layout[value]
  return if entry and entry.keys then entry.keys else {}
end

local function floatsIn(layout: TomlLayout, value: TomlTable): { [string | number]: boolean }
  local entry = layout[value]
  return if entry then entry.floats else {}
end

local function lightOverrideFromToml(
  origin: string,
  value: TomlValue,
  layout: TomlLayout,
  id: string
): LightOverride
  if type(value) ~= 'table' then
    return invalid(
      origin,
      `invalid type for light override \`{id}\`: expected struct RawCustomLightData`
    )
  end

  local raw = value :: TomlTable
  local floats = floatsIn(layout, raw)

  local function uint(key: string, max: number): number?
    return tomlUint(origin, raw[key], floats[key] == true, max, key)
  end

  local function float(key: string): number? return tomlF32(origin, raw[key], key) end

  local function finite(number: number?, key: string)
    if number and not isFinite(number) then invalid(origin, `{key} must be finite`) end
  end

  local red, green, blue = uint('red', U8Max), uint('green', U8Max), uint('blue', U8Max)

  local override: LightOverride = {
    redMult = float 'red_mult',
    greenMult = float 'green_mult',
    blueMult = float 'blue_mult',
    hueMult = float 'hue_mult',
    saturationMult = float 'saturation_mult',
    valueMult = float 'value_mult',
    radius = uint('radius', U32Max),
    radiusMult = float 'radius_mult',
    duration = float 'duration',
    durationMult = float 'duration_mult',
  }

  local hue, saturation, brightness = uint('hue', U32Max), float 'saturation', float 'value'

  local flag = raw.flag
  if type(flag) == 'string' then
    override.flags = parseFlags(origin, flag)
  elseif type(flag) == 'table' then
    override.flags = parseFlagTokens(origin, tomlStrings(origin, flag, 'flag'))
  elseif flag ~= nil then
    invalid(origin, 'data did not match any variant of untagged enum RawLightFlag')
  end

  if override.radius and override.radiusMult then
    invalid(origin, 'Fields `radius` and `radius_mult` are mutually exclusive')
  elseif override.duration and override.durationMult then
    invalid(origin, 'Fields `duration` and `duration_mult` are mutually exclusive')
  end

  finite(override.redMult, 'red_mult')
  finite(override.greenMult, 'green_mult')
  finite(override.blueMult, 'blue_mult')
  finite(override.hueMult, 'hue_mult')
  finite(override.saturationMult, 'saturation_mult')
  finite(override.valueMult, 'value_mult')
  finite(override.radiusMult, 'radius_mult')
  finite(override.duration, 'duration')
  finite(override.durationMult, 'duration_mult')
  finite(saturation, 'saturation')
  finite(brightness, 'value')

  if red and green and blue then
    override.color = { red, green, blue, 0 }
  elseif red or green or blue then
    invalid(origin, 'RGB overrides must specify red, green, and blue')
  end

  override.hue = if hue then math.clamp(hue, 0, 360) else nil
  override.saturation = if saturation then fclamp(saturation, 0, 1) else nil
  override.value = if brightness then fclamp(brightness, 0, 1) else nil

  if override.hue and override.hueMult then
    invalid(origin, 'Fields `hue` and `hue_mult` are mutually exclusive')
  elseif override.saturation and override.saturationMult then
    invalid(origin, 'Fields `saturation` and `saturation_mult` are mutually exclusive')
  elseif override.value and override.valueMult then
    invalid(origin, 'Fields `value` and `value_mult` are mutually exclusive')
  end

  return override
end

-- Older configs give ambient colors as HSV, converted to RGB as they're read.
local function rgbFromToml(origin: string, value: TomlValue?, layout: TomlLayout, key: string): Rgb?
  if value == nil then return nil end

  if type(value) ~= 'table' then
    return invalid(origin, `invalid type for \`{key}\`: expected struct RawTypedLightColor`)
  end

  local raw = value :: TomlTable
  local floats = floatsIn(layout, raw)

  local function uint(name: string, max: number): number?
    return tomlUint(origin, raw[name], floats[name] == true, max, name)
  end

  local red, green, blue, hue =
    uint('red', U8Max), uint('green', U8Max), uint('blue', U8Max), uint('hue', U32Max)
  local saturation, brightness =
    tomlF32(origin, raw.saturation, 'saturation'), tomlF32(origin, raw.value, 'value')
  local rgb: { number }?, hsv: { number }?

  if red and green and blue then
    rgb = { red, green, blue }
  elseif red or green or blue then
    invalid(origin, 'RGB overrides must specify red, green, and blue')
  end

  if hue and saturation and brightness then
    hsv = hsvToColor {
      hue = f32(math.clamp(hue, 0, 360)),
      saturation = fclamp(saturation, 0, 1),
      value = fclamp(brightness, 0, 1),
    }
  elseif hue or saturation or brightness then
    invalid(origin, 'HSV ambient colors must specify hue, saturation, and value')
  end

  if rgb and hsv then
    invalid(origin, 'RGB color fields are mutually exclusive with legacy HSV color fields')
  end

  local color: { number } = rgb
    or hsv
    or invalid(origin, 'RGB colors must specify red, green, and blue')
  return { red = color[1], green = color[2], blue = color[3] }
end

local function ambientOverrideFromToml(
  origin: string,
  value: TomlValue,
  layout: TomlLayout,
  id: string
): AmbientOverride
  if type(value) ~= 'table' then
    return invalid(
      origin,
      `invalid type for ambient override \`{id}\`: expected struct CustomCellAmbient`
    )
  end

  local raw = value :: TomlTable

  return {
    ambient = rgbFromToml(origin, raw.ambient, layout, 'ambient'),
    sunlight = rgbFromToml(origin, raw.sunlight, layout, 'sunlight'),
    fog = rgbFromToml(origin, raw.fog, layout, 'fog'),
    fogDensity = tomlF32(origin, raw.fog_density, 'fog_density'),
  }
end

local function configFromToml(text: string): LightConfig
  -- `jess.toml.decode` throws on text that isn't TOML, which reads as a broken lightconfig.toml.
  local decoded, raw, layout = pcall(jess.toml.decode, text)
  if not decoded then return invalid(ConfigOrigin, problem(raw)) end

  local config = defaultConfig()

  for _, key in BoolKeys do
    local value = raw[key]
    if key == 'disable_interior_sun' and type(value) == 'table' then
      config.interiorSunCells = tomlStrings(ConfigOrigin, value, key)
    elseif value ~= nil then
      config.bools[key] = tomlBool(ConfigOrigin, value, key)
    end
  end

  if raw.save_config ~= nil then
    config.bools.save_config = tomlBool(ConfigOrigin, raw.save_config, 'save_config')
  end

  for _, key in FloatKeys do
    config.floats[key] = tomlF32(ConfigOrigin, raw[key], key) or config.floats[key]
  end

  if raw.excluded_plugins ~= nil then
    config.excludedPlugins = tomlStrings(ConfigOrigin, raw.excluded_plugins, 'excluded_plugins')
  end

  if raw.excluded_ids ~= nil then
    config.excludedIds = tomlStrings(ConfigOrigin, raw.excluded_ids, 'excluded_ids')
  end

  local outputDir = raw.output_dir
  if type(outputDir) == 'string' then
    config.outputDir = outputDir
  elseif outputDir ~= nil then
    invalid(ConfigOrigin, 'invalid type for `output_dir`: expected path string')
  end

  local lights = raw.light_overrides
  if type(lights) == 'table' then
    local overrides = lights :: TomlTable
    for _, id in keysIn(layout, overrides) do
      local override = lightOverrideFromToml(ConfigOrigin, overrides[id], layout, id)
      insertOrdered(config.lightOverrides, id, override)
    end
  elseif lights ~= nil then
    invalid(ConfigOrigin, 'invalid type for `light_overrides`: expected a map')
  end

  local ambients = raw.ambient_overrides
  if type(ambients) == 'table' then
    local overrides = ambients :: TomlTable
    for _, id in keysIn(layout, overrides) do
      local override = ambientOverrideFromToml(ConfigOrigin, overrides[id], layout, id)
      insertOrdered(config.ambientOverrides, id, override)
    end
  elseif ambients ~= nil then
    invalid(ConfigOrigin, 'invalid type for `ambient_overrides`: expected a map')
  end

  return config
end

local TomlEscapes: { [string]: string } = {
  ['\b'] = '\\b',
  ['\t'] = '\\t',
  ['\n'] = '\\n',
  ['\f'] = '\\f',
  ['\r'] = '\\r',
  ['"'] = '\\"',
  ['\\'] = '\\\\',
}

local TomlControls = '[%z\1-\8\10-\31\127]'

local function tomlEscaped(text: string): string
  local escaped = string.gsub(
    text,
    '[%c\\"]',
    function(character: string): string
      return TomlEscapes[character] or string.format('\\u%04X', string.byte(character))
    end
  )

  return '"' .. escaped .. '"'
end

-- toml_writer's choices: a basic string when nothing needs escaping, else a literal one if it can be.
local function tomlQuoted(text: string): string
  local needsEscapes = string.find(text, TomlControls)
  if not needsEscapes and not string.find(text, '[\\"]') then return '"' .. text .. '"' end

  if not needsEscapes and not string.find(text, '\'', 1, true) then return '\'' .. text .. '\'' end

  return tomlEscaped(text)
end

local function tomlKey(key: string): string
  if key ~= '' and not string.find(key, '[^%w_%-]') then return key end

  return tomlQuoted(key)
end

local function tomlFloat(value: number): string
  if isNan(value) then
    buffer.writef32(scratch, 0, value)
    return if buffer.readu32(scratch, 0) >= 0x80000000 then '-nan' else 'nan'
  end

  if value == 0 then return if 1 / value < 0 then '-0.0' else '0.0' end
  if isFinite(value) and value % 1 == 0 then return displayF32(value) .. '.0' end

  return displayF32(value)
end

local function tomlArray(strings: { string }): string
  local items = {}
  for _, text in strings do
    table.insert(items, tomlQuoted(text))
  end

  if #items < 2 then return '[' .. table.concat(items) .. ']' end

  local lines = { '[' }
  for _, item in items do
    table.insert(lines, '    ' .. item .. ',')
  end

  table.insert(lines, ']')
  return table.concat(lines, '\n')
end

type TomlSection = { path: { string }?, lines: { string }, hasChildren: boolean }

local function tomlPut(section: TomlSection, key: string, value: string)
  table.insert(section.lines, tomlKey(key) .. ' = ' .. value)
end

local function tomlPutFloat(section: TomlSection, key: string, value: number?)
  if value then tomlPut(section, key, tomlFloat(value)) end
end

local function tomlPutInteger(section: TomlSection, key: string, value: number?)
  if value then tomlPut(section, key, integer(value)) end
end

local function renderSection(section: TomlSection): string
  local text = ''
  local path = section.path

  if path then
    local keys = {}
    for _, part in path do
      table.insert(keys, tomlKey(part))
    end

    text = '[' .. table.concat(keys, '.') .. ']\n'
  end

  for _, line in section.lines do
    text ..= line .. '\n'
  end

  return text
end

-- toml::to_string_pretty: the root's values, then each table in the order it was started, leaving
-- out tables with nothing of their own that only hold other tables.
local function configToToml(config: LightConfig): string
  local sections: { TomlSection } = {}

  local function section(path: { string }?): TomlSection
    local new: TomlSection = { path = path, lines = {}, hasChildren = false }
    table.insert(sections, new)
    return new
  end

  local function putRgb(entry: TomlSection, id: string, name: string, color: Rgb?)
    if not color then return end

    local child = section { 'ambient_overrides', id, name }
    entry.hasChildren = true
    tomlPut(child, 'red', integer(color.red))
    tomlPut(child, 'green', integer(color.green))
    tomlPut(child, 'blue', integer(color.blue))
  end

  local root = section(nil)

  for _, key in BoolKeys do
    if key == 'disable_interior_sun' and config.interiorSunCells then
      tomlPut(root, key, tomlArray(config.interiorSunCells))
    else
      tomlPut(root, key, tostring(config.bools[key]))
    end
  end

  for _, key in FloatKeys do
    tomlPut(root, key, tomlFloat(config.floats[key]))
  end

  tomlPut(root, 'excluded_plugins', tomlArray(config.excludedPlugins))
  tomlPut(root, 'excluded_ids', tomlArray(config.excludedIds))

  local lights = section { 'light_overrides' }

  for _, id in config.lightOverrides.keys do
    local override = config.lightOverrides.values[id]
    local entry = section { 'light_overrides', id }
    lights.hasChildren = true

    local color = override.color
    if color then
      tomlPut(entry, 'red', integer(color[1]))
      tomlPut(entry, 'green', integer(color[2]))
      tomlPut(entry, 'blue', integer(color[3]))
    end

    tomlPutFloat(entry, 'red_mult', override.redMult)
    tomlPutFloat(entry, 'green_mult', override.greenMult)
    tomlPutFloat(entry, 'blue_mult', override.blueMult)
    tomlPutInteger(entry, 'hue', override.hue)
    tomlPutFloat(entry, 'saturation', override.saturation)
    tomlPutFloat(entry, 'value', override.value)
    tomlPutFloat(entry, 'hue_mult', override.hueMult)
    tomlPutFloat(entry, 'saturation_mult', override.saturationMult)
    tomlPutFloat(entry, 'value_mult', override.valueMult)
    tomlPutInteger(entry, 'radius', override.radius)
    tomlPutFloat(entry, 'radius_mult', override.radiusMult)
    tomlPutFloat(entry, 'duration', override.duration)
    tomlPutFloat(entry, 'duration_mult', override.durationMult)

    local flags = override.flags
    if not flags then continue end

    local names = flagNames(flags)
    if #names == 0 then
      tomlPut(entry, 'flag', tomlQuoted 'NONE')
    elseif #names == 1 then
      tomlPut(entry, 'flag', tomlQuoted(names[1]))
    else
      tomlPut(entry, 'flag', tomlArray(names))
    end
  end

  local ambients = section { 'ambient_overrides' }

  for _, id in config.ambientOverrides.keys do
    local override = config.ambientOverrides.values[id]
    local entry = section { 'ambient_overrides', id }
    ambients.hasChildren = true

    putRgb(entry, id, 'ambient', override.ambient)
    putRgb(entry, id, 'sunlight', override.sunlight)
    putRgb(entry, id, 'fog', override.fog)
    tomlPutFloat(entry, 'fog_density', override.fogDensity)
  end

  if config.outputDir then tomlPut(root, 'output_dir', tomlQuoted(config.outputDir)) end
  tomlPut(root, 'save_config', tostring(config.bools.save_config))

  local rendered = {}
  for _, current in sections do
    if #current.lines == 0 and (not current.path or current.hasChildren) then continue end

    table.insert(rendered, renderSection(current))
  end

  return table.concat(rendered, '\n')
end

return function(state: { [string]: unknown }?, context: Context?)
  local shared: { [string]: unknown } = state or {}
  local options: Options = (shared.s3lightfixes :: Options?) or {}
  shared.s3lightfixes = options

  local openmw: Context = context
    or error 'Failed to read configuration file! S3LightFixes needs an OpenMW config.'

  local notificationsOff = false

  local function notify(message: string)
    if not notificationsOff then print(message) end
  end

  local userConfigDir = openmw.userConfigDir
  local configFile = jess.fs.join(userConfigDir, 'openmw.cfg')
  -- Plugins are read, and the output written, in the encoding OpenMW reads them in.
  local encoding: string = openmw.encoding or 'win1252'

  -- The original finds lightconfig.toml ignoring case, but always saves it lowercase.
  local function findLightConfig(): string?
    if not jess.fs.isDir(userConfigDir) then return nil end

    for _, name in jess.fs.list(userConfigDir) do
      if string.lower(name) == ConfigName then return jess.fs.join(userConfigDir, name) end
    end

    return nil
  end

  local configPath = findLightConfig()
  local configMissing = not configPath
  local config = if configPath
    then configFromToml(jess.fs.read(configPath) :: string)
    else defaultConfig()
  local bools, floats = config.bools, config.floats

  if options.dryRun ~= nil then
    bools.dry_run = options.dryRun
    bools.validate_config = bools.validate_config and not options.dryRun
  end

  if options.validateConfig ~= nil then
    bools.validate_config = options.validateConfig
    bools.dry_run = bools.dry_run and not options.validateConfig
  end

  if bools.dry_run and bools.validate_config then
    error 'dry_run and validate_config cannot both be true'
  end

  floats.standard_hue = f32(options.standardHue or floats.standard_hue)
  floats.standard_saturation = f32(options.standardSaturation or floats.standard_saturation)
  floats.standard_value = f32(options.standardValue or floats.standard_value)
  floats.standard_radius = f32(options.standardRadius or floats.standard_radius)
  floats.colored_hue = f32(options.coloredHue or floats.colored_hue)
  floats.colored_saturation = f32(options.coloredSaturation or floats.colored_saturation)
  floats.colored_value = f32(options.coloredValue or floats.colored_value)
  floats.colored_radius = f32(options.coloredRadius or floats.colored_radius)
  floats.duration_mult = f32(options.durationMult or floats.duration_mult)

  -- An option set to false turns its setting off, so only a missing one leaves the config's.
  local function optionOr(option: boolean?, setting: boolean): boolean
    if option == nil then return setting end

    return option
  end

  bools.disable_flickering = optionOr(options.disableFlickering, bools.disable_flickering)
  bools.disable_pulse = optionOr(options.disablePulse, bools.disable_pulse)
  bools.disable_negative_lights =
    optionOr(options.disableNegativeLights, bools.disable_negative_lights)
  bools.auto_enable = bools.auto_enable or options.autoEnable == true
  bools.no_notifications = bools.no_notifications or options.noNotifications == true
  bools.debug = bools.debug or options.debug == true
  notificationsOff = bools.no_notifications

  if not bools.validate_config then
    local output = options.output

    if output then
      if not jess.fs.isDir(output) then
        error(`WARNING: The requested output path {output} does not exist! Terminating.`)
      end

      config.outputDir = output
    elseif not config.outputDir then
      config.outputDir = openmw.dataLocal or jess.fs.cwd()
    end
  end

  -- Saving the config writes the options' exclusions into it, so each is kept once, where it first
  -- appears; plugin patterns match ignoring case, so they're compared ignoring case too.
  local function withoutRepeats(
    list: { string },
    additions: { string }?,
    key: (string) -> string
  ): { string }
    local seen: { [string]: boolean }, kept: { string } = {}, {}

    for _, source in { list, additions or {} } do
      for _, entry in source do
        local entryKey = key(entry)
        if seen[entryKey] then continue end

        seen[entryKey] = true
        table.insert(kept, entry)
      end
    end

    return kept
  end

  local function sameId(id: string): string return id end

  config.excludedIds = withoutRepeats(config.excludedIds, options.excludedIds, sameId)
  config.excludedPlugins =
    withoutRepeats(config.excludedPlugins, options.excludedPlugins, string.lower)

  -- Option overrides are checked as lightconfig.toml's are, under their snake_case names.
  local function asToml(value: unknown): TomlValue
    if type(value) ~= 'table' then return value :: TomlValue end

    local result: TomlTable = {}
    for key, item in value :: { [unknown]: unknown } do
      if type(key) == 'number' then return value :: TomlValue end

      local name = string.gsub(
        tostring(key),
        '%u',
        function(letter: string): string return '_' .. string.lower(letter) end
      )

      result[name] = asToml(item)
    end

    return result
  end

  local function overrideOption<T>(
    option: { id: string },
    kind: string,
    read: (string, TomlValue, TomlLayout, string) -> T
  ): (string, T)
    local fields = asToml(option) :: TomlTable
    fields.id = nil

    return option.id, read(`s3lightfixes: {kind} {option.id}: `, fields, {}, option.id)
  end

  for _, option in options.lightOverrides or {} do
    insertOrdered(
      config.lightOverrides,
      overrideOption(option, 'light override', lightOverrideFromToml)
    )
  end

  for _, option in options.ambientOverrides or {} do
    insertOrdered(
      config.ambientOverrides,
      overrideOption(option, 'ambient override', ambientOverrideFromToml)
    )
  end

  local sunOption = options.disableInteriorSun
  if type(sunOption) == 'table' then
    bools.disable_interior_sun = false
    config.interiorSunCells = sunOption
  elseif sunOption ~= nil then
    bools.disable_interior_sun = sunOption
    config.interiorSunCells = nil
  end

  local writesConfig = configMissing or bools.save_config or options.updateLightConfig == true
  if writesConfig and not bools.dry_run and not bools.validate_config then
    jess.fs.write(jess.fs.join(userConfigDir, ConfigName), configToToml(config))
  end

  local CaseInsensitive: RegexOptions = { caseInsensitive = true }

  local compileErrors: { string } = {}

  -- `jess.regex.new` throws on a pattern Rust's regex crate rejects. The original reports every bad
  -- pattern before it stops, so each is caught here and collected.
  local function compile(pattern: string, kind: string): Regex?
    local compiled, regex = pcall(jess.regex.new, pattern, CaseInsensitive)
    if compiled then return regex end

    local message = `Couldn't compile {kind} regex: {pattern}: {regex}`
    notify(message)
    table.insert(compileErrors, message)
    return nil
  end

  local function compileAll(patterns: { string }, kind: string): { Regex }
    local compiled: { Regex } = {}
    for _, pattern in patterns do
      local regex = compile(pattern, kind)
      if regex then table.insert(compiled, regex) end
    end

    return compiled
  end

  local function compileMatchers<T>(overrides: OrderedMap<T>, kind: string): { Matcher<T> }
    local matchers: { Matcher<T> } = {}
    for _, id in overrides.keys do
      local regex = compile(id, kind)
      if regex then table.insert(matchers, { regex = regex, value = overrides.values[id] }) end
    end

    return matchers
  end

  local excludedIds = compileAll(config.excludedIds, 'excluded id')
  local excludedPlugins = compileAll(config.excludedPlugins, 'excluded plugin')
  local lightOverrides = compileMatchers(config.lightOverrides, 'light override')
  local interiorSunCells = compileAll(config.interiorSunCells or {}, 'interior sun cell')
  local ambientOverrides = compileMatchers(config.ambientOverrides, 'ambient override')

  if #compileErrors > 0 then error(table.concat(compileErrors, '\n')) end

  local function matchesAny(patterns: { Regex }, text: string): boolean
    for _, pattern in patterns do
      if pattern:isMatch(text) then return true end
    end

    return false
  end

  local function lightOverrideFor(id: string): LightOverride?
    for _, matcher in lightOverrides do
      if matcher.regex:isMatch(id) then return matcher.value end
    end

    return nil
  end

  local Flickers = bit32.bor(FlagBits.FLICKER, FlagBits.FLICKER_SLOW)
  local Pulses = bit32.bor(FlagBits.PULSE, FlagBits.PULSE_SLOW)

  local function scaledU32(value: number, multiplier: number): number
    return toInteger(fmax(f32(f32(value) * multiplier), 0), 0, U32Max)
  end

  local function scaledI32(value: number, multiplier: number): number
    return toInteger(f32(f32(value) * multiplier), I32Min, I32Max)
  end

  local function scaledU8(value: number, multiplier: number): number
    return toInteger(fclamp(f32(value * multiplier), 0, 255), 0, U8Max)
  end

  local function sameColor(left: { number }, right: { number }): boolean
    return left[1] == right[1]
      and left[2] == right[2]
      and left[3] == right[3]
      and left[4] == right[4]
  end

  local function lightChanges(before: LightValues, after: LightValues): { string }
    local changes = {}

    if not sameColor(before.color, after.color) then
      table.insert(changes, `color {debugArray(before.color)} -> {debugArray(after.color)}`)
    end

    if before.radius ~= after.radius then
      table.insert(changes, `radius {integer(before.radius)} -> {integer(after.radius)}`)
    end

    if before.time ~= after.time then
      table.insert(changes, `duration {integer(before.time)} -> {integer(after.time)}`)
    end

    if before.flags ~= after.flags then
      table.insert(changes, `flags {flagsDebug(before.flags)} -> {flagsDebug(after.flags)}`)
    end

    return changes
  end

  -- A channel set by the override, else scaled by the global multiplier unless the override gives
  -- the whole color.
  local function overriddenChannel(
    channel: number,
    multiplier: number?,
    replacement: number?,
    useGlobal: boolean,
    globalMultiplier: number
  ): number
    if multiplier then return f32(channel * multiplier) end
    if replacement then return replacement end
    if useGlobal then return f32(channel * globalMultiplier) end

    return channel
  end

  -- A duration of -1 is infinite, and stays so.
  local function overriddenTime(time: number, override: LightOverride): number
    if time == -1 then return time end
    if override.durationMult then return scaledI32(time, override.durationMult) end
    if override.duration then return toInteger(override.duration, I32Min, I32Max) end

    return scaledI32(time, floats.duration_mult)
  end

  local function overriddenRadius(
    radius: number,
    override: LightOverride,
    globalMultiplier: number
  ): number
    if override.radiusMult then return scaledU32(radius, override.radiusMult) end
    if override.radius then return override.radius end

    return scaledU32(radius, globalMultiplier)
  end

  local function processLight(id: string, light: LightValues): { string }
    local before: LightValues = {
      time = light.time,
      radius = light.radius,
      color = table.clone(light.color),
      flags = light.flags,
    }

    if bools.disable_negative_lights and bit32.band(light.flags, FlagBits.NEGATIVE) ~= 0 then
      light.flags = bit32.band(light.flags, bit32.bnot(FlagBits.NEGATIVE))
      light.radius = 0
      light.color = { 0, 0, 0, 0 }
      return lightChanges(before, light)
    end

    if bools.disable_flickering then light.flags = bit32.band(light.flags, bit32.bnot(Flickers)) end
    if bools.disable_pulse then light.flags = bit32.band(light.flags, bit32.bnot(Pulses)) end

    local hsv = colorToHsv(light.color)
    local degrees = positiveDegrees(hsv.hue)
    local kind = if degrees >= 14 and degrees <= 64 then 'standard_' else 'colored_'
    local radius, hue = floats[kind .. 'radius'], floats[kind .. 'hue']
    local saturation, value = floats[kind .. 'saturation'], floats[kind .. 'value']

    local override = lightOverrideFor(string.lower(id))
    if not override then
      hsv.hue = f32(hsv.hue * hue)
      hsv.saturation = f32(hsv.saturation * saturation)
      hsv.value = f32(hsv.value * value)
      light.radius = scaledU32(light.radius, radius)
      if light.time ~= -1 then light.time = scaledI32(light.time, floats.duration_mult) end

      light.color = hsvToColor(hsv)
      return lightChanges(before, light)
    end

    local useGlobal = not override.color
    if override.color then hsv = colorToHsv(override.color) end

    local hueReplacement = if override.hue then f32(math.clamp(override.hue, 0, 360)) else nil
    hsv.hue = overriddenChannel(hsv.hue, override.hueMult, hueReplacement, useGlobal, hue)
    hsv.saturation = overriddenChannel(
      hsv.saturation,
      override.saturationMult,
      override.saturation,
      useGlobal,
      saturation
    )
    hsv.value = overriddenChannel(hsv.value, override.valueMult, override.value, useGlobal, value)

    light.time = overriddenTime(light.time, override)
    light.radius = overriddenRadius(light.radius, override, radius)

    if override.flags then
      -- CAN_CARRY decides whether a light is an item; changing it breaks saves and inventories.
      local carry = bit32.band(light.flags, FlagBits.CAN_CARRY)
      light.flags = bit32.bor(bit32.band(override.flags, bit32.bnot(FlagBits.CAN_CARRY)), carry)
    end

    light.color = hsvToColor(hsv)
    if override.redMult then light.color[1] = scaledU8(light.color[1], override.redMult) end
    if override.greenMult then light.color[2] = scaledU8(light.color[2], override.greenMult) end
    if override.blueMult then light.color[3] = scaledU8(light.color[3], override.blueMult) end

    return lightChanges(before, light)
  end

  if bools.validate_config then
    print(`Validated {jess.fs.join(userConfigDir, ConfigName)} successfully`)
    options.report = { validated = true }
    return
  end

  local outputDir = config.outputDir :: string

  if bools.debug then
    print(
      'S3LightFixes config:',
      outputDir,
      #config.lightOverrides.keys,
      #config.ambientOverrides.keys
    )
  end

  local content = openmw:content()
  if #content == 0 then error 'No plugins were found in openmw.cfg! No lights to fix!' end

  local FixableExtensions: { [string]: boolean } =
    { esp = true, esm = true, omwaddon = true, omwgame = true }

  local function isFixablePlugin(path: string): boolean
    local stem, extension = string.match(jess.fs.fileName(path) or '', '^(.*)%.([^%.]*)$')
    if not jess.fs.exists(path) or not stem or not extension then return false end
    if stem == '' or string.lower(stem) == 's3lightfixes' then return false end

    return FixableExtensions[string.lower(extension)] or false
  end

  -- Cells only matter when something can relight them; otherwise they're never read.
  local needsCells = bools.disable_interior_sun or #interiorSunCells > 0 or #ambientOverrides > 0

  -- A plugin is named by its file, which can differ in case from how openmw.cfg spells it.
  local entries: { loadOrder.Content } = {}
  for index = #content, 1, -1 do
    local path = content[index].path
    if not path or not isFixablePlugin(path) then continue end
    if matchesAny(excludedPlugins, jess.fs.fileName(path) or '') then continue end

    table.insert(entries, { name = jess.fs.fileName(path) or path, path = path })
  end

  -- Cells are only relit, so their references are never read.
  local opened = loadOrder.open(openmw, {
    entries = entries,
    types = if needsCells then { 'LIGH', 'CELL' } else { 'LIGH' },
    cell = { references = false },
    encoding = encoding,
  })

  for _, failure in opened.failures do
    print(
      `[ WARNING ]: Plugin {failure.path}: could not be loaded due to error: {failure.problem}. Continuing light fixes without this mod .  . . Everything will be okay. Yes, it's still working.\n`
    )
  end

  -- Plugins come back in order, without the ones that failed.
  local plugins = opened.plugins

  local lights: { Light }, cells: { Cell }, masters: { Master } = {}, {}, {}
  local usedIds: { [string]: boolean }, logs: { RecordLog } = {}, {}
  local objects = 0

  local function cellChanges(before: AtmosphereData, after: AtmosphereData): { string }
    local changes = {}

    if not sameColor(before.ambientColor, after.ambientColor) then
      table.insert(
        changes,
        `ambient {debugArray(before.ambientColor)} -> {debugArray(after.ambientColor)}`
      )
    end

    if not sameColor(before.sunlightColor, after.sunlightColor) then
      table.insert(
        changes,
        `sunlight {debugArray(before.sunlightColor)} -> {debugArray(after.sunlightColor)}`
      )
    end

    if not sameColor(before.fogColor, after.fogColor) then
      table.insert(changes, `fog {debugArray(before.fogColor)} -> {debugArray(after.fogColor)}`)
    end

    -- f32::EPSILON
    if math.abs(f32(before.fogDensity - after.fogDensity)) > 1.1920929e-07 then
      table.insert(
        changes,
        `fog_density {displayF32(before.fogDensity)} -> {displayF32(after.fogDensity)}`
      )
    end

    return changes
  end

  local function isInterior(flags: string): boolean
    return string.find(' ' .. flags .. ' ', '[%s|]IS_INTERIOR[%s|]') ~= nil
  end

  local function rgba(color: Rgb): { number } return { color.red, color.green, color.blue, 0 } end

  -- Whether anything replaced the cell's lighting.
  local function processCell(atmosphere: AtmosphereData, id: string): boolean
    local replaced = false

    if bools.disable_interior_sun or matchesAny(interiorSunCells, id) then
      atmosphere.sunlightColor = { 0, 0, 0, 0 }
      replaced = true
    end

    for _, matcher in ambientOverrides do
      if not matcher.regex:isMatch(id) then continue end

      local override = matcher.value
      if override.ambient then
        atmosphere.ambientColor = rgba(override.ambient)
        replaced = true
      end

      if override.fog then
        atmosphere.fogColor = rgba(override.fog)
        replaced = true
      end

      if override.sunlight then
        atmosphere.sunlightColor = rgba(override.sunlight)
        replaced = true
      end

      if override.fogDensity then
        atmosphere.fogDensity = override.fogDensity
        replaced = true
      end
    end

    return replaced
  end

  local function bytes4(list: { number }): { number }
    return { list[1], list[2], list[3], list[4] }
  end

  -- The cell with its lighting replaced, if anything replaced it.
  local function relitCell(
    cell: Cell,
    atmosphere: AtmosphereData,
    id: string,
    plugin: string
  ): Cell?
    local before: AtmosphereData = {
      ambientColor = bytes4(atmosphere.ambientColor),
      sunlightColor = bytes4(atmosphere.sunlightColor),
      fogColor = bytes4(atmosphere.fogColor),
      fogDensity = atmosphere.fogDensity,
    }

    local after: AtmosphereData = {
      ambientColor = before.ambientColor,
      sunlightColor = before.sunlightColor,
      fogColor = before.fogColor,
      fogDensity = before.fogDensity,
    }

    if not processCell(after, id) then return nil end

    local changes = cellChanges(before, after)
    if #changes > 0 then
      table.insert(logs, { kind = 'CELL', plugin = plugin, id = id, changes = changes })
    end

    local mapColor = cell.mapColor

    return {
      type = 'Cell',
      flags = cell.flags,
      name = cell.name,
      data = { flags = cell.data.flags, grid = { cell.data.grid[1], cell.data.grid[2] } },
      region = cell.region,
      mapColor = if mapColor then bytes4(mapColor) else nil,
      atmosphereData = after,
      references = {},
    }
  end

  local function fixedLight(light: Light, plugin: string): Light
    local data = light.data

    local values: LightValues = {
      time = data.time,
      radius = data.radius,
      color = bytes4(data.color),
      flags = flagsFromJess(data.flags),
    }

    local changes = processLight(light.id, values)
    if #changes > 0 then
      table.insert(logs, { kind = 'LIGH', plugin = plugin, id = light.id, changes = changes })
    end

    return {
      type = 'Light',
      flags = light.flags,
      id = light.id,
      name = light.name,
      script = light.script,
      mesh = light.mesh,
      icon = light.icon,
      sound = light.sound,
      data = {
        weight = data.weight,
        value = data.value,
        time = values.time,
        radius = values.radius,
        color = values.color,
        flags = flagsToJess(values.flags),
      },
    }
  end

  for _, loaded in plugins do
    local plugin = loaded.plugin
    local used = 0

    if needsCells then
      for index = 0, plugin:count() - 1 do
        if plugin:tag(index) ~= 'CELL' then continue end

        local cell = (plugin:decode(index) :: unknown) :: Cell
        local atmosphere = cell.atmosphereData
        if not atmosphere or not isInterior(cell.data.flags) then continue end

        local id = string.lower(cell.name)
        if usedIds[id] or matchesAny(excludedIds, id) then continue end

        local relit = relitCell(cell, atmosphere, id, loaded.name)
        if not relit then continue end

        table.insert(cells, relit)
        usedIds[id] = true
        used += 1
      end
    end

    for index = 0, plugin:count() - 1 do
      if plugin:tag(index) ~= 'LIGH' then continue end

      local light = (plugin:decode(index) :: unknown) :: Light
      local id = string.lower(light.id)
      if usedIds[id] or matchesAny(excludedIds, id) then continue end

      usedIds[id] = true
      table.insert(lights, fixedLight(light, loaded.name))
      used += 1
    end

    if used == 0 then continue end

    table.insert(masters, 1, { name = loaded.name, size = loaded.size })
    objects += used
  end

  local function logLines(): { string }
    local changedCells, changedLights = 0, 0
    for _, log in logs do
      if log.kind == 'CELL' then
        changedCells += 1
      else
        changedLights += 1
      end
    end

    local lines = {
      `# S3LightFixes {Version}`,
      `# config: {configFile}`,
      `# output: {jess.fs.join(outputDir, PluginName)}`,
      `# content files: {#content}`,
      `# loaded plugins: {#plugins}`,
      `# masters: {#masters}`,
      `# changed cells: {changedCells}`,
      `# changed lights: {changedLights}`,
    }

    for _, log in logs do
      local changes = table.concat(log.changes, ', ')
      table.insert(
        lines,
        `{log.kind} {debugString(log.id)} from {debugString(log.plugin)}: {changes}`
      )
    end

    return lines
  end

  if bools.dry_run then
    print('Dry run: no files written\n' .. table.concat(logLines(), '\n'))
    options.report = { dryRun = true, masters = #masters, objects = objects, logs = logs }
    return
  end

  if #masters == 0 then
    error 'The generated plugin was not found to have any master files! It\'s empty! Try running lightfixes again using the S3L_DEBUG environment variable'
  end

  -- Lights sorted by id (stably, by bytes), then cells in the order they were made.
  local order: { [Light]: number } = {}
  for index, light in lights do
    order[light] = index
  end

  table.sort(lights, function(left: Light, right: Light): boolean
    if left.id == right.id then return order[left] < order[right] end

    return bytesLess(left.id, right.id)
  end)

  local generated = tes3.new('plugin', { encoding = encoding })
  generated:setAuthor 'S3'
  generated:setDescription 'Plugin generated by s3-lightfixes'
  generated:setMasters(masters)

  for _, light in lights do
    generated:insertValue(generated:count(), (light :: unknown) :: tes3.Value)
  end

  for _, cell in cells do
    generated:insertValue(generated:count(), (cell :: unknown) :: tes3.Value)
  end

  -- Inserting a record leaves the header's count as it was.
  generated:updateRecordCount()

  local dataLocal = openmw.dataLocal
  if dataLocal then
    local old = jess.fs.join(dataLocal, PluginName)
    -- `jess.fs.remove` throws when the file can't be removed; like the original, the run goes on and
    -- writes the new plugin.
    if jess.fs.isFile(old) then pcall(jess.fs.remove, old) end
  end

  local output = jess.fs.join(outputDir, PluginName)
  if jess.fs.exists(outputDir) and not jess.fs.isDir(outputDir) then
    local cwd = jess.fs.cwd()
    print(
      `WARNING: Couldn't use {outputDir} as an output directory, as it isn't a directory. Using the current working directory, {cwd}, instead!`
    )
    output = jess.fs.join(cwd, PluginName)
  elseif not jess.fs.exists(outputDir) then
    jess.fs.createDir(outputDir)
  end

  jess.fs.write(output, generated:save())

  local function autoEnable(): boolean
    for _, entry in content do
      if entry.name == PluginName then return true end
    end

    -- A plugin that went outside the data folders can't be activated: OpenMW wouldn't find it.
    local added, refusal = loadOrder.activate(openmw, PluginName)
    if added == nil then
      notify(`Refusing to auto-enable {PluginName}: {problem(refusal)}`)
      return false
    end

    -- `save` backs openmw.cfg up first, and throws without changing it when it can't write it.
    local saved, written = pcall(openmw.save, openmw)
    if not saved then
      notify(problem(written))
      return false
    end

    notify(
      `Wrote selected OpenMW config at {configFile} successfully! Backup saved at {written.backup}.`
    )
    return true
  end

  local enabled = bools.auto_enable and autoEnable()

  local log = table.concat(logLines(), '\n')
  print(log)
  jess.fs.write(jess.fs.join(userConfigDir, LogName), log .. '\n')

  if enabled then
    notify(`S3LightFixes.omwaddon generated, enabled, and saved in {outputDir}`)
  else
    notify(`S3LightFixes.omwaddon generated and saved in {outputDir}`)
  end

  options.report = {
    output = output,
    masters = #masters,
    objects = objects,
    lights = #lights,
    cells = #cells,
    enabled = enabled,
    logs = logs,
  }
end
