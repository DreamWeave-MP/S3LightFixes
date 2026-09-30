// S3LightFixes's hero: a Velothi hall of 676 Morrowind lights, and the program retuning every one.
//
// Every light here is a record from Morrowind.esm as S3LightFixes 0.5.0 logs it with the default
// lightconfig.toml. Vanilla, the torches, paper lanterns, the blue glass lantern and the candles
// flicker the way OpenMW flickers them, stepped at 15 Hz towards random targets; the broken Dwemer
// neon flickers twice as fast; and a negative light pools darkness on the floor. The S3LightFixes
// lantern hangs over the hall, its cyan, magenta, yellow and black panes throwing four beams as it
// turns, and a sheet of its light runs down the hall. Each light the sheet passes becomes what the
// plugin makes of it: the arithmetic's own colour, the longer radius, no flicker, and the negative
// light gone dark. Its log line rises from it as it changes, and the sheet counts to 676, the lights
// a run over Morrowind and both expansions changes, before it races off into the dark.
//
// Pointing down the hall moves the sheet there, to compare before and after at any depth. The
// stone, the fixtures and the dust are lit by the same lights in the shaders, so a light's radius is
// the pool you see. The scene renders to a half-float target; a bright pass and blurs make the
// bloom, and the composite applies ACES tone mapping, dithering, and a scrim behind the hero's text.
// Colours come from the site's tokens. Nothing runs off screen or in a hidden tab, the resolution
// drops if frames run slow, and under prefers-reduced-motion one frame is drawn, the sheet halfway.
// Until the first frame, and without WebGL, the lantern logo the stylesheet draws stands in.

import * as THREE from './vendor/three.module.min.js';

// Records ------------------------------------------------------------------------------------------

// Color, radius, duration and flags before and after, from the log. Radii are game units; 128 of
// them are one unit here. `y` is where each kind hangs, `rate` how fast it flickers.
const KINDS = [
  {
    id: 'light_com_torch_01', vanilla: [245, 140, 40], fixed: [140, 74, 46], radius: [256, 307], duration: [150, 375],
    flags: ['DYNAMIC | CAN_CARRY | FIRE | FLICKER_SLOW', 'DYNAMIC | CAN_CARRY | FIRE'], y: 0.55, rate: 1.05,
  },
  {
    id: 'light_de_paper_lantern_01', vanilla: [245, 140, 40], fixed: [140, 74, 46], radius: [256, 307],
    flags: ['DYNAMIC | FLICKER_SLOW', 'DYNAMIC'], y: 0.95, rate: 0.95,
  },
  {
    id: 'light_de_lantern_02', vanilla: [0, 128, 255], fixed: [18, 98, 178], radius: [256, 281], duration: [150, 375],
    flags: ['DYNAMIC | CAN_CARRY | FIRE | FLICKER_SLOW', 'DYNAMIC | CAN_CARRY | FIRE'], y: 0.62, rate: 1.15,
  },
  {
    id: 'light_com_candle_07', vanilla: [245, 140, 40], fixed: [140, 74, 46], radius: [223, 267],
    flags: ['DYNAMIC | FIRE | FLICKER_SLOW', 'DYNAMIC | FIRE'], y: -0.72, rate: 1.25,
  },
];
const NEON = {
  id: 'light_dwrv_neonbroke01', vanilla: [205, 145, 63], fixed: [117, 75, 52], radius: [256, 307],
  flags: ['DYNAMIC | FLICKER', 'DYNAMIC'],
};
const DARK = {
  id: 'dark_256_d_01', vanilla: [255, 255, 255], fixed: [0, 0, 0], radius: [256, 0],
  flags: ['DYNAMIC | NEGATIVE', 'DYNAMIC'],
};

// The hall runs down -z. Pilasters stand every `spacing`, each carrying a light, 338 a side: 676
// lights, one of them the neon and one the negative light.
const HALL = {
  spacing: 2.6, half: 2.2, floor: -1.9, spring: 1.3, first: -2.6, perSide: 338, face: 1.95, lightX: 1.73,
  near: 22, start: 3, length: 150,
};
const NEON_SLOT = 3;      // on the right wall
const DARK_SLOT = 4;      // on the left wall
const CAMERA_X = 0.8;     // the camera stands right of the hall's middle, the text's wall farther off
const LANTERN = { z: -3.4, height: 1.25 };
const SHEET = { start: -1.7, visible: -58, end: HALL.first - HALL.spacing * (HALL.perSide + 1) };
const FIXED_GAIN = 1.55;  // the fixed colours are darker by design; the hall is exposed for both

// The stutter the neon shows now and then, as intervals of its beat: short, short, long, short.
const STUTTER = [1, 1, 2, 1];
const STUTTER_BEAT = 0.42;

const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;

// For tests only: ?s3lf-at=<seconds> steps the scene at 60 Hz to that moment, the same every time,
// and holds it there with the governor off; ?s3lf-sign=<seconds> puts the neon sign up at a moment.
const TEST = (() => {
  const params = new URLSearchParams(location.search);
  const at = parseFloat(params.get('s3lf-at'));
  if (!Number.isFinite(at)) return null;
  const sign = parseFloat(params.get('s3lf-sign'));
  return { at: Math.max(0, at), sign: Number.isFinite(sign) ? sign : null };
})();

function cssColor(name, fallback) {
  const raw = getComputedStyle(document.documentElement).getPropertyValue(name).trim();
  const color = new THREE.Color(fallback);
  if (raw) {
    try { color.setStyle(raw); } catch { /* an unparsable token keeps the fallback */ }
  }
  return color;
}

function linear([r, g, b]) {
  return new THREE.Color().setRGB(r / 255, g / 255, b / 255, THREE.SRGBColorSpace);
}

// The kind of light on a pilaster, the same sum the shaders do: -1 for the neon's and the dark one's.
function slotKind(k, side) {
  if (side > 0 && k === NEON_SLOT) return -1;
  if (side < 0 && k === DARK_SLOT) return -1;
  const h = (k * 37 + (side > 0 ? 17 : 5)) % 23;
  return h < 10 ? 0 : h < 16 ? 1 : h < 20 ? 3 : 2;
}

function slotPosition(k, side, kind) {
  return new THREE.Vector3(side * HALL.lightX, KINDS[kind].y, HALL.first - k * HALL.spacing);
}

function random(seed) {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const smooth = (edge0, edge1, x) => {
  const t = Math.min(1, Math.max(0, (x - edge0) / (edge1 - edge0)));
  return t * t * (3 - 2 * t);
};
const easeInOut = (t) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);

// OpenMW's flicker, as components/sceneutil/lightcontroller.cpp runs it: fifteen times a second the
// brightness steps towards a random target between 0.25 and 1, and picks another on arrival. The
// neon's targets stay at least a quarter apart and are held a moment, so it never flashes quicker
// than three times a second.
class Flicker {
  constructor(speed, seed) {
    this.speed = speed;
    this.next = random(seed);
    this.brightness = 0.675;
    this.phase = 0.25 + this.next() * 0.75;
    this.ticks = 0;
    this.hold = 0;
  }
  update(dt) {
    this.ticks = dt * 15 * 0.25 + this.ticks * 0.75;
    if (this.hold > 0) {
      this.hold -= dt;
      return this.brightness;
    }
    const step = this.ticks * this.speed;
    this.brightness += this.brightness >= this.phase ? -step : step;
    if (Math.abs(this.brightness - this.phase) < this.speed) {
      let target = this.phase;
      while (Math.abs(target - this.brightness) < 0.25) target = 0.25 + this.next() * 0.75;
      this.phase = target;
      this.hold = 0.12;
    }
    this.brightness = Math.min(1, Math.max(0.25, this.brightness));
    return this.brightness;
  }
}

// Shaders ------------------------------------------------------------------------------------------

const CONSTANTS = /* glsl */ `
  #define SPACING ${HALL.spacing.toFixed(3)}
  #define FIRST_Z ${HALL.first.toFixed(3)}
  #define LIGHT_X ${HALL.lightX.toFixed(3)}
  #define PER_SIDE ${HALL.perSide.toFixed(1)}
  #define NEON_K ${NEON_SLOT.toFixed(1)}
  #define DARK_K ${DARK_SLOT.toFixed(1)}
  #define HALF ${HALL.half.toFixed(3)}
  #define FLOOR_Y ${HALL.floor.toFixed(3)}
  #define SPRING_Y ${HALL.spring.toFixed(3)}
  #define FIXED_GAIN ${FIXED_GAIN.toFixed(3)}
`;

// The lights, shared by the stone, the fixtures, the flames and the dust, so they all agree.
const LIGHTS = /* glsl */ `
  ${CONSTANTS}
  uniform float uTime;
  uniform float uSheet;
  uniform float uPulse;
  uniform vec3 uVanilla[4];
  uniform vec3 uFixed[4];
  uniform vec2 uRadius[4];
  uniform float uHeight[4];
  uniform float uRate[4];
  uniform float uLightGain;

  float hash1(float n) { return fract(sin(mod(n, 289.0) * 12.9898) * 43758.5453); }

  float slotKind(float k, float side) {
    if (side > 0.0 && abs(k - NEON_K) < 0.5) return -1.0;
    if (side < 0.0 && abs(k - DARK_K) < 0.5) return -1.0;
    float h = mod(k * 37.0 + (side > 0.0 ? 17.0 : 5.0), 23.0);
    return h < 10.0 ? 0.0 : h < 16.0 ? 1.0 : h < 20.0 ? 3.0 : 2.0;
  }

  // Vanilla flicker, advanced at 15 Hz as OpenMW does: brightness between 0.35 and 1, drifting
  // between random targets about once a second.
  float flicker(float seed, float t, float rate) {
    float tick = floor(t * 15.0) / 15.0;
    float x = tick * rate + seed * 0.618;
    float i = floor(x);
    float f = x - i;
    float a = hash1(i + seed * 31.0);
    float b = hash1(i + 1.0 + seed * 31.0);
    f = f * f * (3.0 - 2.0 * f);
    return 0.35 + 0.65 * mix(a, b, f);
  }

  // How far the sheet has retuned a light at depth z: 0 vanilla, 1 fixed.
  float slotFix(float z) { return smoothstep(0.0, 0.7, z - uSheet); }

  vec3 slotPosition(float k, float side, float kind) {
    return vec3(side * LIGHT_X, uHeight[int(kind)], FIRST_Z - k * SPACING);
  }

  // A light's colour and radius now: x, y, z colour, w radius.
  vec4 slotLight(float k, float side, float kind, float z) {
    int ki = int(kind);
    float fix = slotFix(z);
    float flick = flicker(k * 2.0 + (side > 0.0 ? 1.0 : 0.0), uTime, uRate[ki]);
    vec3 color = mix(uVanilla[ki] * flick, uFixed[ki] * FIXED_GAIN, fix) * uPulse * uLightGain;
    return vec4(color, mix(uRadius[ki].x, uRadius[ki].y, fix));
  }

  float attenuation(float dist, float radius) {
    if (radius < 1e-3 || dist >= radius) return 0.0;
    float x = dist / radius;
    float window = 1.0 - x * x;
    return window * window / (1.0 + 4.0 * x * x);
  }
`;

const SAFE = /* glsl */ `
  vec3 safeNormalize(vec3 v, vec3 fallback) {
    float l = length(v);
    return l > 1e-5 ? v / l : fallback;
  }
`;

const SURFACE_VERTEX = /* glsl */ `
  varying vec3 vWorld;
  varying vec3 vNormal;
  void main() {
    vec4 local = vec4(position, 1.0);
    vec3 n = normal;
    #ifdef USE_INSTANCING
      local = instanceMatrix * local;
      n = mat3(instanceMatrix) * n;
    #endif
    vec4 world = modelMatrix * local;
    vWorld = world.xyz;
    vNormal = mat3(modelMatrix) * n;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

// Stone and iron, lit by every light near them: the pilasters' lights, the neon, the negative light,
// the lantern's flame and its four beams, and the neon sign when it is up. SURFACE picks the stone's
// coursing: 0 floor, 1 wall, 2 vault, 3 pilaster and rib, 4 plain (uAlbedo).
const SURFACE_FRAGMENT = /* glsl */ `
  ${LIGHTS}
  ${SAFE}
  uniform vec3 uAlbedo;
  uniform float uRough;
  uniform vec3 uAmbient;
  uniform vec3 uFog;
  uniform float uFogDensity;
  uniform vec4 uNeon;          // position, radius
  uniform vec3 uNeonColor;
  uniform vec4 uDark;          // position, radius
  uniform vec4 uFlame;         // the lantern's flame: position, radius
  uniform vec3 uFlameColor;
  uniform vec3 uLanternPos;
  uniform float uLanternAngle;
  uniform vec3 uPane[4];
  uniform vec4 uSign;          // the neon sign's middle, and how lit it is
  uniform vec3 uSignColor;
  uniform vec4 uSignDark;      // its black tube: position, strength
  uniform float uSheetOn;
  uniform vec3 uScanColors[3];
  varying vec3 vWorld;
  varying vec3 vNormal;

  float hash2(vec2 p) { return fract(sin(dot(mod(p, 289.0), vec2(127.1, 311.7))) * 43758.5453); }
  float noise2(vec2 p) {
    vec2 i = floor(p);
    vec2 f = fract(p);
    vec2 u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), u.x), mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), u.x), u.y);
  }
  float fbm(vec2 p) {
    return noise2(p) * 0.55 + noise2(p * 2.13 + 7.1) * 0.28 + noise2(p * 4.37 + 3.3) * 0.17;
  }

  // Ashlar coursing: 1 on a block's face, 0 in the mortar, and the block's id.
  float blocks(vec2 uv, vec2 size, out vec2 id) {
    vec2 q = uv / size;
    q.x += mod(floor(q.y), 2.0) * 0.5;
    id = floor(q);
    vec2 f = fract(q);
    vec2 e = min(f, 1.0 - f) * size;
    return smoothstep(0.004, 0.035, min(e.x, e.y));
  }

  vec3 bumpNormal(vec3 n, vec3 p, float h, float strength) {
    vec3 dpdx = dFdx(p);
    vec3 dpdy = dFdy(p);
    float dhdx = dFdx(h);
    float dhdy = dFdy(h);
    vec3 r1 = cross(dpdy, n);
    vec3 r2 = cross(n, dpdx);
    float det = dot(dpdx, r1);
    vec3 grad = sign(det) * (dhdx * r1 + dhdy * r2);
    return safeNormalize(abs(det) * n - strength * grad, n);
  }

  vec3 shade(vec3 p, vec3 n, vec3 v, vec3 lp, vec3 color, float radius, vec3 albedo, float rough) {
    vec3 d = lp - p;
    float dist = length(d);
    float att = attenuation(dist, radius);
    if (att <= 0.0) return vec3(0.0);
    vec3 l = d / max(dist, 1e-4);
    float ndl = max(dot(n, l), 0.0);
    vec3 h = safeNormalize(l + v, n);
    float spec = pow(max(dot(n, h), 0.0), mix(72.0, 10.0, rough)) * (1.0 - rough) * 0.8;
    return color * att * ndl * (albedo + spec);
  }

  vec3 beams(vec3 p, vec3 n) {
    vec3 d = p - uLanternPos;
    float dist = length(d);
    vec3 dir = d / max(dist, 1e-4);
    float ndl = max(dot(n, -dir), 0.0);
    float fall = 1.0 / (1.0 + dist * dist * 0.14);
    vec3 sum = vec3(0.0);
    for (int i = 0; i < 4; i++) {
      float a = uLanternAngle + float(i) * 1.5707963;
      vec3 axis = normalize(vec3(sin(a), -0.2, cos(a)));
      float cone = smoothstep(0.93, 0.985, dot(dir, axis));
      sum += uPane[i] * cone;
    }
    return sum * fall * ndl;
  }

  vec3 scanColor(vec3 p) {
    float t = fract((p.x * 0.8 + p.y * 0.55) * 0.9 - uTime * 0.35);
    return t < 0.3 ? uScanColors[0] : t < 0.6 ? uScanColors[1] : t < 0.9 ? uScanColors[2] : vec3(0.0);
  }

  void main() {
    vec3 p = vWorld;
    vec3 n = safeNormalize(vNormal, vec3(0.0, 1.0, 0.0));
    if (!gl_FrontFacing) n = -n;
    vec3 v = safeNormalize(cameraPosition - p, n);

    vec3 albedo = uAlbedo;
    float rough = uRough;
    float h = 0.0;
    float strength = 0.0;
    #if SURFACE < 4
      vec2 id;
      vec2 uv;
      vec2 size;
      #if SURFACE == 0
        uv = vec2(p.x, p.z);
        size = vec2(0.95, 0.72);
      #elif SURFACE == 1
        uv = vec2(p.z, p.y);
        size = vec2(0.84, 0.38);
      #elif SURFACE == 2
        uv = vec2(p.z, atan(p.y - SPRING_Y, p.x) * HALF);
        size = vec2(0.84, 0.42);
      #else
        uv = vec2(p.z + p.x, p.y);
        size = vec2(0.5, 0.34);
      #endif
      float face = blocks(uv, size, id);
      float grain = fbm(uv * 5.3);
      float tone = hash2(id + float(SURFACE) * 17.0);
      albedo = uAlbedo * (0.78 + tone * 0.34) * (0.86 + grain * 0.28);
      albedo = mix(uAlbedo * 0.34, albedo, face);
      h = face * (0.75 + grain * 0.25) + fbm(uv * 17.0) * 0.08;
      strength = 0.012;
      rough = mix(0.95, uRough, face);
    #else
      h = fbm(p.xy * 23.0 + p.z * 11.0) * 0.4;
      strength = 0.004;
    #endif
    n = bumpNormal(n, p, h, strength);
    #if SURFACE == 4
      float rimTerm = 1.0 - max(dot(n, v), 0.0);
      vec3 rimLight = vec3(0.55, 0.32, 0.16) * rimTerm * rimTerm * rimTerm * (1.0 - uRough) * 0.6;
    #else
      vec3 rimLight = vec3(0.0);
    #endif

    vec3 lit = uAmbient * albedo;
    float k0 = floor((FIRST_Z - p.z) / SPACING + 0.5);
    for (int o = -1; o <= 1; o++) {
      float k = k0 + float(o);
      if (k < 0.0 || k >= PER_SIDE) continue;
      for (int s = 0; s < 2; s++) {
        float side = s == 0 ? -1.0 : 1.0;
        float kind = slotKind(k, side);
        if (kind < 0.0) continue;
        vec3 lp = slotPosition(k, side, kind);
        vec4 light = slotLight(k, side, kind, lp.z);
        lit += shade(p, n, v, lp, light.rgb, light.w, albedo, rough);
      }
    }
    lit += shade(p, n, v, uNeon.xyz + vec3(0.0, 0.55, 0.0), uNeonColor * 0.6 * uLightGain, uNeon.w, albedo, rough);
    lit += shade(p, n, v, uNeon.xyz - vec3(0.0, 0.55, 0.0), uNeonColor * 0.6 * uLightGain, uNeon.w, albedo, rough);
    lit += shade(p, n, v, uFlame.xyz, uFlameColor, uFlame.w, albedo, rough);
    lit += shade(p, n, v, uSign.xyz, uSignColor * uSign.w * 4.0, 4.2, albedo, rough);
    lit += beams(p, n) * albedo;
    lit += rimLight;

    // Negative lights take light away, as Morrowind's do.
    float dark = attenuation(length(uDark.xyz - p), uDark.w) * 1.6;
    dark += attenuation(length(uSignDark.xyz - p), 2.2) * uSignDark.w * 1.4;
    lit *= max(0.0, 1.0 - dark);
    lit = max(lit, vec3(0.0));

    // Where the sheet crosses the stone, a line in the lantern's colours.
    float scanD = (p.z - uSheet) / 0.07;
    lit += scanColor(p) * exp(-scanD * scanD) * uSheetOn * 4.5;

    float dist = length(cameraPosition - p);
    float fog = 1.0 - exp2(-uFogDensity * dist);
    gl_FragColor = vec4(mix(lit, uFog, fog), 1.0);
  }
`;

// The fixtures' glowing parts: paper, blue glass and the neon take their light's colour now.
const GLOW_VERTEX = /* glsl */ `
  attribute vec3 aSlot;
  varying vec3 vSlot;
  varying vec3 vWorld;
  varying vec3 vNormal;
  varying vec3 vLocal;
  void main() {
    vSlot = aSlot;
    vLocal = position;
    vec4 local = instanceMatrix * vec4(position, 1.0);
    vec4 world = modelMatrix * local;
    vWorld = world.xyz;
    vNormal = mat3(modelMatrix) * mat3(instanceMatrix) * normal;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

const GLOW_FRAGMENT = /* glsl */ `
  ${LIGHTS}
  ${SAFE}
  uniform vec3 uFog;
  uniform float uFogDensity;
  uniform float uGain;
  uniform float uRibs;
  varying vec3 vSlot;
  varying vec3 vWorld;
  varying vec3 vNormal;
  varying vec3 vLocal;
  void main() {
    vec3 lp = slotPosition(vSlot.x, vSlot.y, vSlot.z);
    vec4 light = slotLight(vSlot.x, vSlot.y, vSlot.z, lp.z);
    vec3 n = safeNormalize(vNormal, vec3(0.0, 0.0, 1.0));
    vec3 v = safeNormalize(cameraPosition - vWorld, n);
    float facing = abs(dot(n, v));
    float ribs = 1.0 - uRibs * (1.0 - smoothstep(0.0, 0.18, abs(fract(vLocal.y * 9.0) - 0.5) * 2.0)) * 0.55;
    vec3 color = light.rgb / uLightGain * uGain * 2.2 * (0.55 + facing * 0.9) * ribs;
    float dist = length(cameraPosition - vWorld);
    float fog = 1.0 - exp2(-uFogDensity * 0.7 * dist);
    gl_FragColor = vec4(mix(color, uFog, fog), 1.0);
  }
`;

// Flames and glows, one quad per light, all 676 of them: the far ones keep at least a pixel or two,
// a line of lights running into the dark.
const FLAME_VERTEX = /* glsl */ `
  ${LIGHTS}
  attribute vec3 aSlot;
  uniform float uPixel;        // pixels per unit at unit depth
  varying vec2 vUv;
  varying vec3 vColor;
  varying float vKind;
  varying float vFix;
  varying float vDepth;
  varying float vSeed;
  void main() {
    float kind = aSlot.z;
    vec3 lp = slotPosition(aSlot.x, aSlot.y, kind);
    vec4 light = slotLight(aSlot.x, aSlot.y, kind, lp.z);
    vFix = slotFix(lp.z);
    vColor = light.rgb / uLightGain;
    vKind = kind;
    vSeed = aSlot.x * 2.0 + aSlot.y;
    vec3 centre = lp;
    vec2 size;
    if (kind < 0.5) { centre.y += 0.12; size = vec2(0.24, 0.42); }
    else if (kind < 1.5) { size = vec2(0.7, 0.7); }
    else if (kind < 2.5) { size = vec2(0.48, 0.48); }
    else { size = vec2(0.46, 0.3); }
    vec4 view = viewMatrix * vec4(centre, 1.0);
    float depth = max(-view.z, 0.01);
    vDepth = depth;
    float least = 2.2 * depth / max(uPixel, 1.0);
    size = max(size, vec2(least));
    view.xy += position.xy * size;
    vUv = position.xy + 0.5;
    gl_Position = projectionMatrix * view;
  }
`;

const FLAME_FRAGMENT = /* glsl */ `
  uniform float uTime;
  uniform vec3 uFog;
  uniform float uFogDensity;
  varying vec2 vUv;
  varying vec3 vColor;
  varying float vKind;
  varying float vFix;
  varying float vDepth;
  varying float vSeed;
  float hash1(float n) { return fract(sin(mod(n, 289.0) * 12.9898) * 43758.5453); }
  float tongue(vec2 uv, float seed, float scale) {
    vec2 p = uv * 2.0 - 1.0;
    float t = uTime * 2.3 + seed * 5.1;
    p.x += sin(p.y * 3.1 + t) * 0.09 * (p.y + 1.0) + sin(t * 1.7) * 0.03;
    float y = clamp(p.y * 0.5 + 0.5, 0.0, 1.0);
    float width = mix(0.55, 0.02, y * y) * scale;
    float body = 1.0 - smoothstep(width * 0.55, width, abs(p.x));
    body *= smoothstep(-1.0, -0.65, p.y) * (1.0 - smoothstep(0.55, 1.0, p.y));
    return body;
  }
  void main() {
    vec2 p = vUv * 2.0 - 1.0;
    float r = length(p);
    float glow = exp(-r * r * 5.0);
    vec3 hot = vec3(1.0, 0.86, 0.62);
    vec3 color;
    if (vKind < 0.5 || vKind > 2.5) {
      float f;
      if (vKind < 0.5) f = tongue(vUv, vSeed, 1.0);
      else {
        f = 0.0;
        for (int i = 0; i < 3; i++) {
          float x = (vUv.x - 0.2 - float(i) * 0.3) / 0.3 + 0.5;
          float y = (vUv.y - (i == 1 ? 0.1 : 0.0)) / 0.9;
          if (x > 0.0 && x < 1.0) f = max(f, tongue(vec2(x, y), vSeed + float(i) * 3.3, 1.0));
        }
      }
      color = mix(vColor * 2.2, hot * 1.6, 0.18 * f * f) * f + vColor * glow * 0.8;
    } else {
      color = vColor * (glow * 1.8 + exp(-r * r * 22.0) * 2.4);
    }
    float fog = 1.0 - exp2(-uFogDensity * 0.35 * vDepth);
    gl_FragColor = vec4(color * (1.0 - fog * 0.85), 1.0);
  }
`;

// Dust in the hall, catching whatever light is near.
const DUST_VERTEX = /* glsl */ `
  ${LIGHTS}
  attribute vec4 aSeed;
  uniform float uPixel;
  uniform vec3 uFlamePos;
  varying vec3 vColor;
  void main() {
    vec3 p = vec3(mix(-2.0, 2.0, aSeed.x), mix(-1.8, 3.0, aSeed.y), mix(1.0, -26.0, aSeed.z));
    float t = uTime * (0.05 + aSeed.w * 0.06);
    p += vec3(sin(t + aSeed.w * 30.0) * 0.4, fract(t * 0.3 + aSeed.x) * 0.9 - 0.45, cos(t * 0.7 + aSeed.y * 20.0) * 0.4);
    vec3 light = vec3(0.0);
    float k0 = floor((FIRST_Z - p.z) / SPACING + 0.5);
    for (int o = -1; o <= 1; o++) {
      float k = k0 + float(o);
      if (k < 0.0 || k >= PER_SIDE) continue;
      for (int s = 0; s < 2; s++) {
        float side = s == 0 ? -1.0 : 1.0;
        float kind = slotKind(k, side);
        if (kind < 0.0) continue;
        vec3 lp = slotPosition(k, side, kind);
        vec4 l = slotLight(k, side, kind, lp.z);
        light += l.rgb * attenuation(length(lp - p), l.w);
      }
    }
    light += vec3(1.0, 0.85, 0.7) * attenuation(length(uFlamePos - p), 4.0);
    vColor = light * 0.35;
    vec4 view = viewMatrix * vec4(p, 1.0);
    gl_Position = projectionMatrix * view;
    gl_PointSize = clamp(0.028 * uPixel / max(-view.z, 0.1), 1.0, 5.0);
  }
`;

const DUST_FRAGMENT = /* glsl */ `
  varying vec3 vColor;
  void main() {
    vec2 p = gl_PointCoord * 2.0 - 1.0;
    float a = max(0.0, 1.0 - dot(p, p));
    gl_FragColor = vec4(vColor * a * a, 1.0);
  }
`;

// The sheet: a thin veil across the hall in the lantern's colours, with the plugin's name and the
// count along its top.
const SHEET_VERTEX = /* glsl */ `
  varying vec2 vUv;
  varying vec3 vWorld;
  void main() {
    vUv = uv;
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorld = world.xyz;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

const SHEET_FRAGMENT = /* glsl */ `
  uniform float uTime;
  uniform float uOpacity;
  uniform vec3 uColors[3];
  uniform vec3 uFog;
  uniform float uFogDensity;
  varying vec2 vUv;
  varying vec3 vWorld;
  void main() {
    float bands = fract(vUv.y * 3.0 + vUv.x * 0.6 - uTime * 0.18);
    vec3 tint = bands < 0.33 ? uColors[0] : bands < 0.66 ? uColors[1] : uColors[2];
    float lines = 0.5 + 0.5 * sin(vUv.y * 420.0 + sin(vUv.x * 9.0 + uTime) * 3.0);
    float edge = smoothstep(0.0, 0.08, vUv.x) * smoothstep(0.0, 0.08, 1.0 - vUv.x);
    vec3 color = tint * (0.04 + lines * 0.07) * edge;
    float dist = length(cameraPosition - vWorld);
    float fog = exp2(-${(0.018).toFixed(3)} * dist);
    gl_FragColor = vec4(color * uOpacity * fog, 1.0);
  }
`;

// The lantern's glass: each pane its colour, brightest where the flame shows through; black is a
// dark mirror.
const PANE_VERTEX = /* glsl */ `
  varying vec3 vWorld;
  varying vec3 vNormal;
  varying vec2 vUv;
  void main() {
    vUv = uv;
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorld = world.xyz;
    vNormal = mat3(modelMatrix) * normal;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

const PANE_FRAGMENT = /* glsl */ `
  ${SAFE}
  uniform vec3 uColor;
  uniform float uBlack;
  uniform float uFlame;
  uniform vec3 uFog;
  varying vec3 vWorld;
  varying vec3 vNormal;
  varying vec2 vUv;
  void main() {
    vec3 n = safeNormalize(vNormal, vec3(0.0, 0.0, 1.0));
    vec3 v = safeNormalize(cameraPosition - vWorld, n);
    float facing = abs(dot(n, v));
    float fresnel = 1.0 - facing;
    vec2 c = vUv - vec2(0.5, 0.42);
    float core = exp(-dot(c, c) * 9.0);
    vec3 glass = uColor * (0.75 + core * 1.9) * uFlame + vec3(1.0, 0.9, 0.75) * core * core * 0.35 * uFlame;
    float streak = smoothstep(0.035, 0.0, abs(vUv.x - vUv.y * 0.35 - 0.2)) * 0.9;
    vec3 black = vec3(0.012, 0.01, 0.018) + vec3(0.5, 0.45, 0.6) * (fresnel * fresnel * 0.5 + streak * 0.25);
    vec3 color = mix(glass + uColor * fresnel * 0.4, black, uBlack);
    gl_FragColor = vec4(color, 1.0);
  }
`;

// The neon, its tube and the sign's tubes: a hot core and a coloured sheath.
const TUBE_VERTEX = /* glsl */ `
  varying vec3 vWorld;
  varying vec3 vNormal;
  void main() {
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorld = world.xyz;
    vNormal = mat3(modelMatrix) * normal;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

const TUBE_FRAGMENT = /* glsl */ `
  ${SAFE}
  uniform vec3 uColor;
  uniform float uIntensity;
  uniform float uBlack;
  uniform vec3 uFog;
  uniform float uFogDensity;
  varying vec3 vWorld;
  varying vec3 vNormal;
  void main() {
    vec3 n = safeNormalize(vNormal, vec3(0.0, 0.0, 1.0));
    vec3 v = safeNormalize(cameraPosition - vWorld, n);
    float facing = abs(dot(n, v));
    vec3 core = mix(uColor, vec3(1.0), 0.28 * facing * facing);
    vec3 color = core * uIntensity * (0.45 + facing * 1.1);
    vec3 black = vec3(0.01, 0.008, 0.014) + vec3(0.35, 0.25, 0.45) * pow(1.0 - facing, 3.0) * 0.6;
    color = mix(color, black, uBlack);
    float dist = length(cameraPosition - vWorld);
    float fog = 1.0 - exp2(-uFogDensity * 0.6 * dist);
    gl_FragColor = vec4(mix(color, uFog, fog), 1.0);
  }
`;

// The lantern's beams, seen in the dust of the hall: soft cones from each coloured pane, brightest
// near the glass and along their axis.
const BEAM_VERTEX = /* glsl */ `
  varying vec3 vWorld;
  varying vec3 vNormal;
  varying float vAlong;
  void main() {
    vAlong = uv.y;
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorld = world.xyz;
    vNormal = mat3(modelMatrix) * normal;
    gl_Position = projectionMatrix * viewMatrix * world;
  }
`;

const BEAM_FRAGMENT = /* glsl */ `
  ${SAFE}
  uniform vec3 uColor;
  uniform float uTime;
  uniform float uIntensity;
  uniform vec3 uAxis;
  uniform vec3 uApex;
  varying vec3 vWorld;
  varying vec3 vNormal;
  varying float vAlong;
  void main() {
    vec3 n = safeNormalize(vNormal, vec3(0.0, 0.0, 1.0));
    vec3 v = safeNormalize(cameraPosition - vWorld, n);
    float edge = abs(dot(n, v));
    float body = edge * edge;
    float from = 1.0 - vAlong;
    vec3 axis = safeNormalize(uAxis, vec3(0.0, 0.0, -1.0));
    float toward = max(0.0, dot(axis, safeNormalize(cameraPosition - uApex, axis)));
    body *= 1.0 - toward * toward * 0.9;
    float fall = exp(-from * 3.2) * smoothstep(0.0, 0.08, from);
    float shimmer = 0.85 + 0.15 * sin(vWorld.y * 7.0 + vWorld.z * 3.0 - uTime * 0.9);
    gl_FragColor = vec4(uColor * uIntensity * fall * body * shimmer * 0.7, 1.0);
  }
`;

// The negative light, drawn: a hole in the air that the dark pours out of.
const VOID_FRAGMENT = /* glsl */ `
  uniform float uTime;
  uniform float uAmount;
  uniform vec3 uRim;
  varying vec2 vUv;
  float pow2(float x) { return x * x; }
  void main() {
    vec2 p = vUv * 2.0 - 1.0;
    float r = length(p);
    float a = atan(p.y, p.x + 1e-5);
    float swirl = 0.5 + 0.5 * sin(a * 5.0 - uTime * 1.3 + r * 9.0);
    float hole = 1.0 - smoothstep(0.18, 0.55, r);
    float rim = exp(-pow2((r - 0.5) * 7.0)) * (0.35 + swirl * 0.4);
    float motes = step(0.93, fract(sin(floor(a * 12.0 + uTime * 0.7) * 91.7) * 331.3)) * smoothstep(0.35, 0.6, r) * (1.0 - smoothstep(0.6, 0.9, r));
    float darkness = clamp(hole * 0.92 + (1.0 - smoothstep(0.3, 1.0, r)) * 0.45, 0.0, 1.0) * uAmount;
    gl_FragColor = vec4(uRim * (rim + motes * 0.6) * uAmount, darkness);
  }
`;

const CARD_VERTEX = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
  }
`;

const CARD_FRAGMENT = /* glsl */ `
  uniform sampler2D tCard;
  uniform float uOpacity;
  varying vec2 vUv;
  void main() {
    vec4 c = texture2D(tCard, vUv);
    gl_FragColor = vec4(c.rgb, c.a * uOpacity);
  }
`;

// Post-processing ----------------------------------------------------------------------------------

const FULLSCREEN_VERTEX = /* glsl */ `
  varying vec2 vUv;
  void main() {
    vUv = uv;
    gl_Position = vec4(position.xy, 0.0, 1.0);
  }
`;

// Any NaN or infinity a driver produces is zeroed and bright values capped before the bloom, which
// would otherwise smear a single bad pixel into a black square.
const SCRUB = /* glsl */ `
  vec3 scrub(vec3 c) {
    if (any(isnan(c)) || any(isinf(c)) || c.r != c.r || c.g != c.g || c.b != c.b) return vec3(0.0);
    return clamp(c, 0.0, 64.0);
  }
`;

const BRIGHT_FRAGMENT = /* glsl */ `
  uniform sampler2D tInput;
  uniform float uThreshold;
  varying vec2 vUv;
  ${SCRUB}
  void main() {
    vec3 c = scrub(texture2D(tInput, vUv).rgb);
    float luma = dot(c, vec3(0.2126, 0.7152, 0.0722));
    gl_FragColor = vec4(c * smoothstep(uThreshold, uThreshold + 0.8, luma), 1.0);
  }
`;

const BLUR_FRAGMENT = /* glsl */ `
  uniform sampler2D tInput;
  uniform vec2 uDirection;
  varying vec2 vUv;
  void main() {
    vec3 sum = texture2D(tInput, vUv).rgb * 0.2270270270;
    sum += texture2D(tInput, vUv + uDirection * 1.3846153846).rgb * 0.3162162162;
    sum += texture2D(tInput, vUv - uDirection * 1.3846153846).rgb * 0.3162162162;
    sum += texture2D(tInput, vUv + uDirection * 3.2307692308).rgb * 0.0702702703;
    sum += texture2D(tInput, vUv - uDirection * 3.2307692308).rgb * 0.0702702703;
    gl_FragColor = vec4(sum, 1.0);
  }
`;

const COPY_FRAGMENT = /* glsl */ `
  uniform sampler2D tInput;
  varying vec2 vUv;
  void main() { gl_FragColor = vec4(texture2D(tInput, vUv).rgb, 1.0); }
`;

// The hall fades to the page's own background behind the hero's words.
const COMPOSITE_FRAGMENT = /* glsl */ `
  uniform sampler2D tScene;
  uniform sampler2D tBloomNear;
  uniform sampler2D tBloomFar;
  uniform float uTime;
  uniform vec4 uText;          // the words' box in uv, x0 y0 x1 y1
  uniform vec4 uFacts;         // the facts strip's
  uniform vec2 uResolution;
  uniform vec3 uBackground;
  uniform float uExposure;
  varying vec2 vUv;
  vec3 aces(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
  }
  float dither(vec2 p) {
    return fract(sin(dot(p + fract(uTime), vec2(12.9898, 78.233))) * 43758.5453) - 0.5;
  }
  ${SCRUB}
  void main() {
    vec3 color = scrub(texture2D(tScene, vUv).rgb);
    color += scrub(texture2D(tBloomNear, vUv).rgb) * 0.75 + scrub(texture2D(tBloomFar, vUv).rgb) * 0.6;
    color = aces(color * uExposure);
    color = pow(color, vec3(1.0 / 2.2));
    vec2 px = vUv * uResolution;
    vec2 outside = max(max(uText.xy * uResolution - px, px - uText.zw * uResolution), vec2(0.0));
    float inside = 1.0 - smoothstep(0.0, 90.0, length(outside));
    vec2 outsideFacts = max(max(uFacts.xy * uResolution - px, px - uFacts.zw * uResolution), vec2(0.0));
    inside = max(inside, (1.0 - smoothstep(0.0, 24.0, length(outsideFacts))) * 0.75);
    color = mix(color, uBackground, inside * 0.8);
    color += dither(gl_FragCoord.xy) / 255.0;
    gl_FragColor = vec4(color, 1.0);
  }
`;

function fullscreenMaterial(fragmentShader, uniforms) {
  return new THREE.ShaderMaterial({ vertexShader: FULLSCREEN_VERTEX, fragmentShader, uniforms, depthTest: false, depthWrite: false });
}

// Geometry -----------------------------------------------------------------------------------------

function hallGeometry(length) {
  const zNear = HALL.start;
  const zFar = zNear - length;
  const floor = new THREE.PlaneGeometry(HALL.half * 2, length, 1, 1);
  floor.rotateX(-Math.PI / 2);
  floor.translate(0, HALL.floor, (zNear + zFar) / 2);
  const wallHeight = HALL.spring - HALL.floor;
  const walls = [-1, 1].map((side) => {
    const wall = new THREE.PlaneGeometry(length, wallHeight, 1, 1);
    wall.rotateY(side > 0 ? -Math.PI / 2 : Math.PI / 2);
    wall.translate(side * HALL.half, (HALL.spring + HALL.floor) / 2, (zNear + zFar) / 2);
    return wall;
  });
  const vault = new THREE.CylinderGeometry(HALL.half, HALL.half, length, 36, 1, true, Math.PI / 2, Math.PI);
  vault.rotateX(Math.PI / 2);
  vault.translate(0, HALL.spring, (zNear + zFar) / 2);
  return { floor, walls, vault };
}

// A pilaster on each side at every light, a rib over the vault between each pair.
function pilasterGeometry() {
  const height = HALL.spring - HALL.floor + 0.2;
  const shaft = new THREE.BoxGeometry(0.46, height, 0.34);
  shaft.translate(0, HALL.floor + height / 2, 0);
  return shaft;
}

function ribGeometry() {
  const rib = new THREE.TorusGeometry(HALL.half - 0.06, 0.13, 8, 40, Math.PI);
  rib.translate(0, HALL.spring, 0);
  return rib;
}

function mergeInto(target, geometry, matrix) {
  const g = geometry.clone();
  if (matrix) g.applyMatrix4(matrix);
  target.push(g.toNonIndexed());
}

function merge(geometries) {
  const count = geometries.reduce((sum, g) => sum + g.attributes.position.count, 0);
  const position = new Float32Array(count * 3);
  const normal = new Float32Array(count * 3);
  let offset = 0;
  for (const g of geometries) {
    position.set(g.attributes.position.array, offset * 3);
    normal.set(g.attributes.normal.array, offset * 3);
    offset += g.attributes.position.count;
  }
  const merged = new THREE.BufferGeometry();
  merged.setAttribute('position', new THREE.BufferAttribute(position, 3));
  merged.setAttribute('normal', new THREE.BufferAttribute(normal, 3));
  return merged;
}

// The iron and wood of each kind of fixture, built on the right wall with the light at the origin;
// the left wall's are mirrored.
function fixtureGeometry(kind) {
  const parts = [];
  const m = new THREE.Matrix4();
  const toWall = HALL.face - HALL.lightX;
  if (kind === 0) {
    mergeInto(parts, new THREE.BoxGeometry(toWall, 0.05, 0.05), m.makeTranslation(toWall / 2, -0.42, 0));
    mergeInto(parts, new THREE.CylinderGeometry(0.075, 0.045, 0.1, 8, 1, true), m.makeTranslation(0.04, -0.38, 0));
    const stick = new THREE.CylinderGeometry(0.03, 0.022, 0.5, 6);
    mergeInto(parts, stick, new THREE.Matrix4().makeTranslation(0.03, -0.2, 0).multiply(new THREE.Matrix4().makeRotationZ(0.16)));
  } else if (kind === 1) {
    mergeInto(parts, new THREE.BoxGeometry(toWall + 0.05, 0.04, 0.04), m.makeTranslation(toWall / 2, 0.52, 0));
    mergeInto(parts, new THREE.CylinderGeometry(0.004, 0.004, 0.3, 3), m.makeTranslation(0, 0.37, 0));
    mergeInto(parts, new THREE.CylinderGeometry(0.19, 0.19, 0.03, 16), m.makeTranslation(0, 0.215, 0));
    mergeInto(parts, new THREE.CylinderGeometry(0.19, 0.19, 0.03, 16), m.makeTranslation(0, -0.215, 0));
  } else if (kind === 2) {
    mergeInto(parts, new THREE.BoxGeometry(toWall + 0.05, 0.04, 0.04), m.makeTranslation(toWall / 2, 0.3, 0));
    mergeInto(parts, new THREE.ConeGeometry(0.17, 0.12, 4), m.makeTranslation(0, 0.22, 0).multiply(new THREE.Matrix4().makeRotationY(Math.PI / 4)));
    mergeInto(parts, new THREE.BoxGeometry(0.22, 0.03, 0.22), m.makeTranslation(0, -0.16, 0));
    for (const [x, z] of [[-0.1, -0.1], [0.1, -0.1], [-0.1, 0.1], [0.1, 0.1]]) {
      mergeInto(parts, new THREE.BoxGeometry(0.025, 0.32, 0.025), m.makeTranslation(x, 0.02, z));
    }
  } else {
    mergeInto(parts, new THREE.BoxGeometry(toWall + 0.3, 0.035, 0.46), m.makeTranslation((toWall - 0.3) / 2 + 0.15, -0.34, 0));
    for (const [x, z, h] of [[-0.02, -0.13, 0.24], [0.04, 0.0, 0.32], [0.0, 0.14, 0.2]]) {
      mergeInto(parts, new THREE.CylinderGeometry(0.035, 0.038, h, 10), m.makeTranslation(x, -0.32 + h / 2, z));
    }
  }
  return merge(parts);
}

// Paper and glass, the parts that glow.
function glowGeometry(kind) {
  if (kind === 1) return new THREE.CylinderGeometry(0.18, 0.18, 0.4, 20, 1, true);
  if (kind === 2) {
    const glass = new THREE.BoxGeometry(0.19, 0.3, 0.19);
    glass.translate(0, 0.02, 0);
    return glass;
  }
  return null;
}

function slotMatrix(k, side, kind) {
  const position = slotPosition(k, side, kind);
  const matrix = new THREE.Matrix4().makeTranslation(position.x, position.y, position.z);
  if (side < 0) matrix.multiply(new THREE.Matrix4().makeRotationY(Math.PI));
  return matrix;
}

// The S3LightFixes lantern, as on the program's icon: a pyramid cap and ring, four panes in cyan,
// magenta, yellow and black, corner posts and a stepped foot.
function lanternGeometry() {
  const iron = [];
  const m = new THREE.Matrix4();
  for (const [x, z] of [[-0.21, -0.21], [0.21, -0.21], [-0.21, 0.21], [0.21, 0.21]]) {
    mergeInto(iron, new THREE.BoxGeometry(0.055, 0.66, 0.055), m.makeTranslation(x, 0, z));
  }
  for (const y of [0.33, -0.33]) {
    for (const [x, z, w, d] of [[0, -0.21, 0.47, 0.045], [0, 0.21, 0.47, 0.045], [-0.21, 0, 0.045, 0.47], [0.21, 0, 0.045, 0.47]]) {
      mergeInto(iron, new THREE.BoxGeometry(w, 0.045, d), m.makeTranslation(x, y, z));
    }
  }
  mergeInto(iron, new THREE.CylinderGeometry(0.07, 0.36, 0.22, 4, 1), new THREE.Matrix4().makeTranslation(0, 0.46, 0).multiply(new THREE.Matrix4().makeRotationY(Math.PI / 4)));
  mergeInto(iron, new THREE.CylinderGeometry(0.05, 0.07, 0.07, 8), m.makeTranslation(0, 0.6, 0));
  mergeInto(iron, new THREE.TorusGeometry(0.1, 0.022, 8, 24), m.makeTranslation(0, 0.72, 0));
  mergeInto(iron, new THREE.CylinderGeometry(0.33, 0.2, 0.1, 4), new THREE.Matrix4().makeTranslation(0, -0.4, 0).multiply(new THREE.Matrix4().makeRotationY(Math.PI / 4)));
  mergeInto(iron, new THREE.CylinderGeometry(0.1, 0.05, 0.12, 8), m.makeTranslation(0, -0.5, 0));
  const panes = [0, 1, 2, 3].map((i) => {
    const pane = new THREE.PlaneGeometry(0.37, 0.6);
    pane.translate(0, 0, 0.2);
    pane.rotateY(i * Math.PI / 2);
    return pane;
  });
  return { iron: merge(iron), panes };
}

// The sign the neon puts up: the lantern in cyan, 676 in magenta, a yellow rule and a black one.
function signStrokes() {
  const circle = [];
  for (let i = 0; i <= 28; i++) {
    const a = Math.PI / 2 - i / 28 * Math.PI * 2;
    circle.push([Math.cos(a) * 0.13, 1.08 + Math.sin(a) * 0.13]);
  }
  const digit = (x, shape) => {
    const l = x - 0.17;
    const r = x + 0.17;
    const t = -0.32;
    const mid = -0.62;
    const b = -0.92;
    return shape === 6 ? [[r, t], [l, t], [l, b], [r, b], [r, mid], [l, mid]] : [[l, t], [r, t], [r, b]];
  };
  return [
    { color: 0, points: circle },
    { color: 0, points: [[-0.34, 0.74], [-0.11, 0.95], [0.11, 0.95], [0.34, 0.74], [-0.34, 0.74]] },
    { color: 0, points: [[-0.3, 0.74], [-0.3, -0.06], [0.3, -0.06], [0.3, 0.74]] },
    { color: 0, points: [[0, 0.74], [0, -0.06]] },
    { color: 0, points: [[-0.3, -0.06], [-0.18, -0.2], [0.18, -0.2], [0.3, -0.06]] },
    { color: 1, points: digit(-0.46, 6) },
    { color: 1, points: digit(0, 7) },
    { color: 1, points: digit(0.46, 6) },
    { color: 2, points: [[-0.72, -1.1], [0.72, -1.1]] },
    { color: 3, points: [[-0.72, -1.24], [0.72, -1.24]] },
  ];
}

function strokeGeometry(points) {
  const path = new THREE.CurvePath();
  for (let i = 1; i < points.length; i++) {
    path.add(new THREE.LineCurve3(new THREE.Vector3(points[i - 1][0], points[i - 1][1], 0), new THREE.Vector3(points[i][0], points[i][1], 0)));
  }
  const segments = Math.max(8, Math.round(path.getLength() * 60));
  return new THREE.TubeGeometry(path, segments, 0.028, 8, false);
}

// Text -------------------------------------------------------------------------------------------

function fontFamily() {
  const raw = getComputedStyle(document.documentElement).getPropertyValue('--dw-font-mono').trim();
  return raw || 'ui-monospace, "DejaVu Sans Mono", monospace';
}

// A card of log lines, as the program prints them, drawn at the page's own pixel scale and shown
// over the finished frame, untouched by the bloom and tone mapping.
function cardCanvas(lines, palette, fontSize, fixedWidth) {
  const scale = Math.min(3, Math.max(1, window.devicePixelRatio || 1));
  const font = `${fontSize}px ${fontFamily()}`;
  const measure = document.createElement('canvas').getContext('2d');
  measure.font = font;
  const pad = Math.round(fontSize * 0.85);
  const lineHeight = Math.round(fontSize * 1.5);
  const textWidth = Math.max(...lines.map((parts) => parts.reduce((sum, [text]) => sum + measure.measureText(text).width, 0)));
  const width = Math.ceil(fixedWidth || textWidth + pad * 2);
  const height = Math.ceil(lines.length * lineHeight + pad * 1.1);
  const canvas = document.createElement('canvas');
  canvas.width = width * scale;
  canvas.height = height * scale;
  const ctx = canvas.getContext('2d');
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.NoColorSpace;
  texture.minFilter = THREE.LinearFilter;
  texture.generateMipmaps = false;
  const draw = (rows) => {
    ctx.setTransform(scale, 0, 0, scale, 0, 0);
    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = 'rgba(12, 8, 16, 0.86)';
    ctx.strokeStyle = palette.line;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.roundRect(0.5, 0.5, width - 1, height - 1, 5);
    ctx.fill();
    ctx.stroke();
    ctx.font = font;
    ctx.textBaseline = 'middle';
    rows.forEach((parts, row) => {
      let x = pad;
      for (const [text, color] of parts) {
        ctx.fillStyle = palette[color] || palette.text;
        ctx.fillText(text, x, pad * 0.55 + lineHeight * (row + 0.5));
        x += ctx.measureText(text).width;
      }
    });
    texture.needsUpdate = true;
  };
  draw(lines);
  return { texture, width, height, draw };
}

const list = (rgb) => `[${rgb.join(', ')}, 0]`;

// The record's line from the log, broken where it would run long.
function recordLines(record, compact) {
  const head = [['LIGH ', 'kind'], [`"${record.id}"`, 'id'], compact ? ['', 'text'] : [' from "Morrowind.esm"', 'text']];
  const color = [['color ', 'text'], [list(record.vanilla), 'old'], [' -> ', 'arrow'], [list(record.fixed), 'new']];
  const radius = [['radius ', 'text'], [String(record.radius[0]), 'old'], [' -> ', 'arrow'], [String(record.radius[1]), 'new']];
  if (record.duration && !compact) radius.push([', duration ', 'text'], [String(record.duration[0]), 'old'], [' -> ', 'arrow'], [String(record.duration[1]), 'new']);
  const flags = [['flags ', 'text'], [record.flags[0], 'old']];
  const flagsTo = [['   -> ', 'arrow'], [record.flags[1], 'new']];
  return compact ? [head, color, radius] : [head, color, radius, flags, flagsTo];
}

// Layout -----------------------------------------------------------------------------------------

function textRects(text) {
  const rects = [];
  const range = document.createRange();
  const walker = document.createTreeWalker(text, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => (node.nodeValue.trim() ? NodeFilter.FILTER_ACCEPT : NodeFilter.FILTER_REJECT),
  });
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    range.selectNodeContents(node);
    for (const rect of range.getClientRects()) rects.push(rect);
  }
  for (const element of text.querySelectorAll('a, button, input, select, img, svg, .dw-command, .dw-badge')) rects.push(element.getBoundingClientRect());
  return rects.filter((rect) => rect.width > 0 && rect.height > 0);
}

// Where the lantern hangs: the largest square clear of the text, beside it, beside the title rows,
// or above it all where the stylesheet leaves room on a phone. Relative to the art, with the box
// the words take.
function placement(root) {
  const hero = root.closest('.dw-hero') || root.parentElement;
  const box = root.getBoundingClientRect();
  const text = hero.querySelector('.dw-hero__text') || hero.querySelector('.dw-shell');
  const strip = hero.querySelector('.dw-strip');
  const shellElement = hero.querySelector('.dw-hero__grid') || hero.querySelector('.dw-shell') || hero;
  const shellStyle = getComputedStyle(shellElement);
  const shellBox = shellElement.getBoundingClientRect();
  const shell = strip ? strip.getBoundingClientRect() : { left: shellBox.left + parseFloat(shellStyle.paddingLeft), right: shellBox.right - parseFloat(shellStyle.paddingRight) };
  const summary = hero.querySelector('.dw-hero__summary');
  const floor = strip ? strip.getBoundingClientRect().top : box.bottom - 24;
  const textOnly = text ? textRects(text) : [];
  const relative = (rect) => ({ left: rect.left - box.left, top: rect.top - box.top, right: rect.right - box.left, bottom: rect.bottom - box.top });
  const words = textOnly.length ? relative({
    left: Math.min(...textOnly.map((r) => r.left)),
    top: Math.min(...textOnly.map((r) => r.top)),
    right: Math.max(...textOnly.map((r) => r.right)),
    bottom: Math.max(...textOnly.map((r) => r.bottom)),
  }) : { left: 0, top: 0, right: 0, bottom: 0 };
  const facts = strip ? relative(strip.getBoundingClientRect()) : { left: 0, top: 0, right: 0, bottom: 0 };
  if (!textOnly.length) return { x: box.width * 0.75, y: box.height * 0.45, size: Math.min(box.width * 0.3, box.height * 0.7), above: false, words, facts };
  const gap = 32;
  const right = Math.max(...textOnly.map((rect) => rect.right));
  const top = Math.min(...textOnly.map((rect) => rect.top));
  const summaryTop = summary ? summary.getBoundingClientRect().top : floor;
  const headRects = textOnly.filter((rect) => rect.bottom <= summaryTop + 1);
  const headRight = headRects.length ? Math.max(...headRects.map((rect) => rect.right)) : right;
  const candidates = [
    { x0: right + gap, x1: shell.right, y0: box.top + 18, y1: floor - 18, above: false },
    { x0: headRight + gap, x1: shell.right, y0: box.top + 12, y1: summaryTop - 12, above: false },
    { x0: shell.left, x1: shell.right, y0: box.top + 10, y1: top - 14, above: true },
  ].map((region) => ({ ...region, size: Math.max(0, Math.min(region.y1 - region.y0, region.x1 - region.x0)) }));
  const best = candidates.reduce((a, b) => (b.size > a.size ? b : a));
  const size = Math.min(best.size * 0.9, 420);
  const x = best.above ? (best.x0 + best.x1) / 2 : Math.min(best.x1 - size / 2, (best.x0 + best.x1) / 2 + (best.x1 - best.x0 - size) * 0.2);
  return { x: x - box.left, y: (best.y0 + best.y1) / 2 - box.top, size, above: best.above, words, facts };
}

// The scene --------------------------------------------------------------------------------------

function mount(root) {
  const canvas = document.createElement('canvas');
  canvas.className = 's3-hero__canvas';
  // Without WebGL 2 the logo the stylesheet draws stays, and nothing is logged.
  let context = null;
  try {
    context = canvas.getContext('webgl2', { antialias: false, alpha: false, powerPreference: 'high-performance' });
  } catch {
    context = null;
  }
  if (!context) return;
  let renderer;
  try {
    renderer = new THREE.WebGLRenderer({ canvas, context, antialias: false, alpha: false });
  } catch {
    return;
  }
  renderer.autoClear = false;
  renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
  root.append(canvas);

  const floatTargets = renderer.extensions.has('EXT_color_buffer_float') || renderer.extensions.has('EXT_color_buffer_half_float');
  const targetType = floatTargets ? THREE.HalfFloatType : THREE.UnsignedByteType;
  const makeTarget = () => new THREE.WebGLRenderTarget(1, 1, { type: targetType, depthBuffer: false });
  const sceneTarget = new THREE.WebGLRenderTarget(1, 1, { type: targetType, samples: 4 });
  const bloomTargets = [makeTarget(), makeTarget(), makeTarget(), makeTarget()];

  const accent = cssColor('--dw-accent', '#f07bd6');
  const background = cssColor('--dw-bg-0', '#140b14');
  const fog = background.clone().multiplyScalar(0.28);
  const cyan = new THREE.Color('#45d4dc');
  const yellow = new THREE.Color('#efd35a');
  const small = Math.min(innerWidth, innerHeight) < 700;
  const quality = { level: small ? 0.85 : 1, slow: 0 };

  const camera = new THREE.PerspectiveCamera(50, 1, 0.05, 1200);
  camera.position.set(CAMERA_X, 0, 0);
  camera.lookAt(CAMERA_X, 0, -1);
  const scene = new THREE.Scene();

  // Uniforms every lit material shares.
  const shared = {
    uTime: { value: 0 },
    uSheet: { value: SHEET.start },
    uPulse: { value: 1 },
    uVanilla: { value: KINDS.map((k) => linear(k.vanilla)) },
    uFixed: { value: KINDS.map((k) => linear(k.fixed)) },
    uRadius: { value: KINDS.map((k) => new THREE.Vector2(k.radius[0] / 128, k.radius[1] / 128)) },
    uHeight: { value: KINDS.map((k) => k.y) },
    uRate: { value: KINDS.map((k) => k.rate) },
    uFog: { value: fog },
    uFogDensity: { value: small ? 0.075 : 0.06 },
    uLightGain: { value: 7 },
  };
  const lighting = {
    ...shared,
    uAmbient: { value: new THREE.Color(0.016, 0.014, 0.02) },
    uNeon: { value: new THREE.Vector4(0, 0, 0, NEON.radius[0] / 128) },
    uNeonColor: { value: new THREE.Color() },
    uDark: { value: new THREE.Vector4(-1.25, HALL.floor + 0.55, HALL.first - DARK_SLOT * HALL.spacing, DARK.radius[0] / 128) },
    uFlame: { value: new THREE.Vector4(0, 0, LANTERN.z, 4.2) },
    uFlameColor: { value: new THREE.Color(1.0, 0.86, 0.72).multiplyScalar(1.6) },
    uLanternPos: { value: new THREE.Vector3(0, 0, LANTERN.z) },
    uLanternAngle: { value: 0 },
    uPane: { value: [cyan.clone().multiplyScalar(1.5), accent.clone().multiplyScalar(1.5), yellow.clone().multiplyScalar(1.3), new THREE.Color(-0.4, -0.4, -0.4)] },
    uSign: { value: new THREE.Vector4(0, 0, -9, 0) },
    uSignColor: { value: new THREE.Color(0.9, 0.75, 0.95) },
    uSignDark: { value: new THREE.Vector4(0, 0, -9, 0) },
    uSheetOn: { value: 1 },
    uScanColors: { value: [cyan, accent, yellow] },
  };
  const surface = (kind, albedo, rough) => new THREE.ShaderMaterial({
    vertexShader: SURFACE_VERTEX,
    fragmentShader: SURFACE_FRAGMENT,
    uniforms: { ...lighting, uAlbedo: { value: new THREE.Color(albedo) }, uRough: { value: rough } },
    defines: { SURFACE: kind },
    side: kind === 2 || kind === 1 ? THREE.DoubleSide : THREE.FrontSide,
  });

  // The hall.
  const hallLength = small ? 110 : HALL.length;
  const hall = hallGeometry(hallLength);
  const sandstone = '#8a7358';
  scene.add(new THREE.Mesh(hall.floor, surface(0, '#6f604e', 0.55)));
  for (const wall of hall.walls) scene.add(new THREE.Mesh(wall, surface(1, sandstone, 0.8)));
  scene.add(new THREE.Mesh(hall.vault, surface(2, '#7c6a55', 0.85)));
  const bays = Math.ceil(hallLength / HALL.spacing) + 2;
  const pilasters = new THREE.InstancedMesh(pilasterGeometry(), surface(3, '#9a8264', 0.7), bays * 2);
  const ribs = new THREE.InstancedMesh(ribGeometry(), surface(3, '#9a8264', 0.7), bays);
  for (let k = 0; k < bays; k++) {
    const z = HALL.first - k * HALL.spacing;
    for (const side of [-1, 1]) {
      pilasters.setMatrixAt(k * 2 + (side > 0 ? 1 : 0), new THREE.Matrix4().makeTranslation(side * (HALL.half - 0.02), 0, z));
    }
    ribs.setMatrixAt(k, new THREE.Matrix4().makeTranslation(0, 0, z));
  }
  scene.add(pilasters, ribs);

  // The fixtures near enough to see, and a flame or glow for every light in the hall.
  const iron = surface(4, '#2a2622', 0.4);
  const wood = surface(4, '#3b2a1c', 0.9);
  const wax = surface(4, '#d8c9a8', 0.6);
  const glowMaterial = (ribsAmount, gain) => new THREE.ShaderMaterial({
    vertexShader: GLOW_VERTEX,
    fragmentShader: GLOW_FRAGMENT,
    uniforms: { ...shared, uGain: { value: gain }, uRibs: { value: ribsAmount } },
    side: THREE.DoubleSide,
  });
  const nearSlots = [0, 1, 2, 3].map(() => []);
  const allSlots = [];
  for (let k = 0; k < HALL.perSide; k++) {
    for (const side of [-1, 1]) {
      const kind = slotKind(k, side);
      if (kind < 0) continue;
      allSlots.push([k, side, kind]);
      if (k < HALL.near) nearSlots[kind].push([k, side, kind]);
    }
  }
  for (let kind = 0; kind < 4; kind++) {
    const slots = nearSlots[kind];
    const material = kind === 3 ? wax : kind === 0 ? wood : iron;
    const fixtures = new THREE.InstancedMesh(fixtureGeometry(kind), material, slots.length);
    const glowShape = glowGeometry(kind);
    const glows = glowShape ? new THREE.InstancedMesh(glowShape, glowMaterial(kind === 1 ? 1 : 0, kind === 1 ? 0.9 : 1.1), slots.length) : null;
    const slotData = new Float32Array(slots.length * 3);
    slots.forEach(([k, side], i) => {
      const matrix = slotMatrix(k, side, kind);
      fixtures.setMatrixAt(i, matrix);
      if (glows) glows.setMatrixAt(i, matrix);
      slotData.set([k, side, kind], i * 3);
    });
    scene.add(fixtures);
    if (glows) {
      glows.geometry.setAttribute('aSlot', new THREE.InstancedBufferAttribute(slotData, 3));
      scene.add(glows);
    }
  }
  const flameGeometry = new THREE.InstancedBufferGeometry();
  flameGeometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array([-0.5, -0.5, 0, 0.5, -0.5, 0, 0.5, 0.5, 0, -0.5, 0.5, 0]), 3));
  flameGeometry.setIndex([0, 1, 2, 0, 2, 3]);
  const flameSlots = new Float32Array(allSlots.length * 3);
  allSlots.forEach((slot, i) => flameSlots.set(slot, i * 3));
  flameGeometry.setAttribute('aSlot', new THREE.InstancedBufferAttribute(flameSlots, 3));
  flameGeometry.instanceCount = allSlots.length;
  const flameUniforms = { ...shared, uPixel: { value: 600 } };
  const flames = new THREE.Mesh(flameGeometry, new THREE.ShaderMaterial({
    vertexShader: FLAME_VERTEX,
    fragmentShader: FLAME_FRAGMENT,
    uniforms: flameUniforms,
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  }));
  flames.frustumCulled = false;
  flames.renderOrder = 2;
  scene.add(flames);

  // The broken Dwemer neon, on the right wall.
  const neonZ = HALL.first - NEON_SLOT * HALL.spacing;
  const neonPosition = new THREE.Vector3(HALL.face - 0.1, 0.45, neonZ);
  lighting.uNeon.value.set(neonPosition.x, neonPosition.y, neonPosition.z, NEON.radius[0] / 128);
  const tubeUniforms = { uColor: { value: new THREE.Color() }, uIntensity: { value: 1 }, uBlack: { value: 0 }, uFog: shared.uFog, uFogDensity: shared.uFogDensity };
  const neonTube = new THREE.Mesh(new THREE.CylinderGeometry(0.034, 0.034, 1.3, 12), new THREE.ShaderMaterial({ vertexShader: TUBE_VERTEX, fragmentShader: TUBE_FRAGMENT, uniforms: tubeUniforms }));
  neonTube.position.copy(neonPosition);
  const brass = surface(4, '#8a6a36', 0.35);
  const neonParts = [];
  const m = new THREE.Matrix4();
  for (const y of [0.7, -0.7]) mergeInto(neonParts, new THREE.CylinderGeometry(0.055, 0.055, 0.12, 12), m.makeTranslation(0, y, 0));
  for (const y of [0.62, -0.62]) mergeInto(neonParts, new THREE.BoxGeometry(0.1, 0.04, 0.05), m.makeTranslation(0.06, y, 0));
  mergeInto(neonParts, new THREE.BoxGeometry(0.02, 1.62, 0.2), m.makeTranslation(0.1, 0, 0));
  const neonFixture = new THREE.Mesh(merge(neonParts), brass);
  neonFixture.position.copy(neonPosition);
  const haloUniforms = { uColor: { value: new THREE.Color() } };
  const neonHalo = new THREE.Mesh(new THREE.PlaneGeometry(0.7, 1.9), new THREE.ShaderMaterial({
    vertexShader: CARD_VERTEX,
    fragmentShader: /* glsl */ `
      uniform vec3 uColor;
      varying vec2 vUv;
      void main() {
        vec2 p = (vUv - 0.5) * vec2(2.0, 2.0);
        float across = p.x * p.x * 9.0;
        float along = max(0.0, abs(p.y) - 0.68) * 5.0;
        gl_FragColor = vec4(uColor * exp(-across - along * along) * 0.55, 1.0);
      }`,
    uniforms: haloUniforms,
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  }));
  neonHalo.position.copy(neonPosition).add(new THREE.Vector3(-0.06, 0, 0));
  neonHalo.renderOrder = 3;
  scene.add(neonTube, neonFixture, neonHalo);

  // The negative light, drawn as what it does.
  const voidUniforms = { uTime: shared.uTime, uAmount: { value: 1 }, uRim: { value: accent.clone().lerp(new THREE.Color(0.4, 0.3, 0.9), 0.5).multiplyScalar(0.5) } };
  const voidMesh = new THREE.Mesh(new THREE.PlaneGeometry(1.3, 1.3), new THREE.ShaderMaterial({
    vertexShader: CARD_VERTEX,
    fragmentShader: VOID_FRAGMENT,
    uniforms: voidUniforms,
    transparent: true,
    depthWrite: false,
  }));
  voidMesh.position.set(lighting.uDark.value.x, lighting.uDark.value.y, lighting.uDark.value.z);
  voidMesh.renderOrder = 3;
  scene.add(voidMesh);

  // The lantern.
  const lanternShape = lanternGeometry();
  const lantern = new THREE.Group();
  const lanternIron = new THREE.Mesh(lanternShape.iron, surface(4, '#1c1a20', 0.3));
  lantern.add(lanternIron);
  const paneColors = [cyan, accent, yellow, new THREE.Color(0, 0, 0)];
  const panes = lanternShape.panes.map((geometry, i) => {
    const pane = new THREE.Mesh(geometry, new THREE.ShaderMaterial({
      vertexShader: PANE_VERTEX,
      fragmentShader: PANE_FRAGMENT,
      uniforms: { uColor: { value: paneColors[i] }, uBlack: { value: i === 3 ? 1 : 0 }, uFlame: { value: 1 }, uFog: shared.uFog },
      side: THREE.DoubleSide,
    }));
    lantern.add(pane);
    return pane;
  });
  const beamShape = new THREE.ConeGeometry(2.1, 7.5, 32, 1, true);
  beamShape.translate(0, -7.5 / 2, 0);
  beamShape.rotateX(-Math.PI / 2);
  beamShape.rotateX(-0.2);
  const beams = [0, 1, 2].map((i) => {
    const beam = new THREE.Mesh(beamShape, new THREE.ShaderMaterial({
      vertexShader: BEAM_VERTEX,
      fragmentShader: BEAM_FRAGMENT,
      uniforms: { uColor: { value: paneColors[i] }, uTime: shared.uTime, uIntensity: { value: 0.13 }, uAxis: { value: new THREE.Vector3() }, uApex: { value: new THREE.Vector3() } },
      transparent: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
      side: THREE.DoubleSide,
    }));
    beam.rotation.y = i * Math.PI / 2;
    beam.renderOrder = 7;
    lantern.add(beam);
    return beam;
  });
  const chain = new THREE.Mesh(new THREE.CylinderGeometry(0.012, 0.012, 1, 4), iron);
  scene.add(lantern, chain);
  const core = new THREE.Mesh(new THREE.PlaneGeometry(0.42, 0.42), new THREE.ShaderMaterial({
    vertexShader: CARD_VERTEX,
    fragmentShader: /* glsl */ `
      varying vec2 vUv;
      uniform float uGain;
      void main() {
        vec2 p = vUv * 2.0 - 1.0;
        float r2 = dot(p, p);
        vec3 c = vec3(1.0, 0.9, 0.78) * (exp(-r2 * 7.0) * 1.2 + exp(-r2 * 40.0) * 3.0) * uGain;
        gl_FragColor = vec4(c, 1.0);
      }`,
    uniforms: { uGain: { value: 1 } },
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  }));
  core.renderOrder = 4;
  scene.add(core);

  // The sheet.
  const palette = {
    text: '#d9ccd9', id: '#efd35a', kind: '#45d4dc', old: '#b9a5b3', new: `#${accent.getHexString(THREE.SRGBColorSpace)}`, arrow: '#8d7d8a',
    line: `#${accent.clone().multiplyScalar(0.7).getHexString(THREE.SRGBColorSpace)}`, comment: '#9d8fa0',
  };
  const sheetUniforms = { uTime: shared.uTime, uOpacity: { value: 1 }, uColors: { value: [cyan, accent, yellow] }, uFog: shared.uFog, uFogDensity: shared.uFogDensity };
  const sheetHeight = HALL.spring + HALL.half - HALL.floor;
  const sheet = new THREE.Mesh(new THREE.PlaneGeometry(HALL.half * 2, sheetHeight), new THREE.ShaderMaterial({
    vertexShader: SHEET_VERTEX,
    fragmentShader: SHEET_FRAGMENT,
    uniforms: sheetUniforms,
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
    side: THREE.DoubleSide,
  }));
  sheet.renderOrder = 5;
  scene.add(sheet);
  // Dust.
  const dustCount = small ? 260 : 520;
  const dustGeometry = new THREE.BufferGeometry();
  const dustSeeds = new Float32Array(dustCount * 4);
  const seedRandom = random(676);
  for (let i = 0; i < dustSeeds.length; i++) dustSeeds[i] = seedRandom();
  dustGeometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(dustCount * 3), 3));
  dustGeometry.setAttribute('aSeed', new THREE.BufferAttribute(dustSeeds, 4));
  const dustUniforms = { ...shared, uPixel: flameUniforms.uPixel, uFlamePos: { value: new THREE.Vector3() } };
  const dust = new THREE.Points(dustGeometry, new THREE.ShaderMaterial({
    vertexShader: DUST_VERTEX,
    fragmentShader: DUST_FRAGMENT,
    uniforms: dustUniforms,
    transparent: true,
    depthWrite: false,
    blending: THREE.AdditiveBlending,
  }));
  dust.frustumCulled = false;
  dust.renderOrder = 6;
  scene.add(dust);

  // The neon sign, hidden until it is asked for.
  const signColors = [cyan, accent, yellow, new THREE.Color(0, 0, 0)];
  const sign = new THREE.Group();
  sign.position.set(CAMERA_X - 1.3, 0.02, -5.6);
  sign.scale.setScalar(0.92);
  const strokes = signStrokes().map((stroke) => {
    const geometry = strokeGeometry(stroke.points);
    const uniforms = { uColor: { value: signColors[stroke.color] }, uIntensity: { value: 2.4 }, uBlack: { value: stroke.color === 3 ? 1 : 0 }, uFog: shared.uFog, uFogDensity: shared.uFogDensity };
    const mesh = new THREE.Mesh(geometry, new THREE.ShaderMaterial({ vertexShader: TUBE_VERTEX, fragmentShader: TUBE_FRAGMENT, uniforms }));
    mesh.visible = false;
    sign.add(mesh);
    return { mesh, count: geometry.index.count, color: stroke.color, uniforms };
  });
  scene.add(sign);

  // The overlay, drawn over the finished frame in the page's pixels: one log card at a time beside
  // the light it is about, a leader line to that light, and the log's header with the count.
  const overlayScene = new THREE.Scene();
  const overlayCamera = new THREE.OrthographicCamera(0, 1, 0, -1, -1, 1);
  const overlayMaterial = (texture) => new THREE.ShaderMaterial({
    vertexShader: CARD_VERTEX,
    fragmentShader: CARD_FRAGMENT,
    uniforms: { tCard: { value: texture }, uOpacity: { value: 0 } },
    transparent: true,
    depthTest: false,
    depthWrite: false,
  });
  const plane = new THREE.PlaneGeometry(1, 1);
  plane.translate(0.5, -0.5, 0);
  const leaderMaterial = () => new THREE.ShaderMaterial({
    vertexShader: CARD_VERTEX,
    fragmentShader: /* glsl */ `
      uniform vec3 uColor;
      uniform float uOpacity;
      void main() { gl_FragColor = vec4(uColor, uOpacity); }`,
    uniforms: { uColor: { value: new THREE.Color(palette.new).convertLinearToSRGB() }, uOpacity: { value: 0 } },
    transparent: true,
    depthTest: false,
    depthWrite: false,
  });
  const compactCards = () => width < 700;
  const cardCache = new Map();
  function cardFor(key, record, extra) {
    const compact = compactCards();
    const cacheKey = `${key}:${compact}`;
    if (!cardCache.has(cacheKey)) {
      const lines = recordLines(record, compact);
      if (extra) lines.push(extra);
      cardCache.set(cacheKey, cardCanvas(lines, palette, compact ? 10 : 12));
    }
    return cardCache.get(cacheKey);
  }
  const cards = [];
  function spawnCard(position, key, record, extra, life = 2.8) {
    for (const card of cards) card.life = Math.min(card.life, time - card.born + 0.3);
    const card = cardFor(key, record, extra);
    const mesh = new THREE.Mesh(plane, overlayMaterial(card.texture));
    const leaderGeometry = new THREE.BufferGeometry();
    leaderGeometry.setAttribute('position', new THREE.BufferAttribute(new Float32Array(6), 3));
    const leader = new THREE.Line(leaderGeometry, leaderMaterial());
    overlayScene.add(mesh, leader);
    cards.push({ mesh, leader, anchor: position.clone(), born: time, life, width: card.width, height: card.height });
  }
  const header = cardCanvas([[['# S3LightFixes 0.5.0', 'comment']], [['# changed lights: 676', 'comment']]], palette, 12);
  const headerMesh = new THREE.Mesh(plane, overlayMaterial(header.texture));
  overlayScene.add(headerMesh);
  let shownCount = -1;
  function setCount(count) {
    if (count === shownCount) return;
    shownCount = count;
    header.draw([[['# S3LightFixes 0.5.0', 'comment']], [['# changed lights: ', 'comment'], [String(count), count ? 'new' : 'comment']]]);
  }
  const screen = new THREE.Vector3();
  function placeCard(card, age) {
    screen.copy(card.anchor).project(camera);
    const sx = (screen.x + 1) / 2 * width;
    const sy = (1 - screen.y) / 2 * height - age * 10;
    if (screen.z > 1) return;
    const words = place.words;
    const lanternBox = { left: place.x - place.size * 0.25, right: place.x + place.size * 0.25, top: place.y - place.size * 0.45, bottom: place.y + place.size * 0.2 };
    const spots = [
      [sx + 22, sy - card.height * 0.5],
      [sx + 22, sy - card.height - 22],
      [sx + 22, sy + 18],
      [sx - 22 - card.width, sy - card.height - 22],
      [sx - 22 - card.width, sy - card.height * 0.5],
      [sx - 22 - card.width, sy + 18],
    ];
    const wordsBox = { left: words.left - 8, right: words.right + 8, top: words.top - 8, bottom: words.bottom + 8 };
    const corners = [[-0.8, 1.25], [0.8, 1.25], [-0.8, -1.3], [0.8, -1.3]].map(([cx, cy]) => {
      screen.set(cx, cy, 0).applyMatrix4(sign.matrixWorld).project(camera);
      return [(screen.x + 1) / 2 * width, (1 - screen.y) / 2 * height];
    });
    const signBox = { left: Math.min(...corners.map((c) => c[0])), right: Math.max(...corners.map((c) => c[0])), top: Math.min(...corners.map((c) => c[1])), bottom: Math.max(...corners.map((c) => c[1])) };
    const headerBox = { left: headerMesh.position.x - 8, right: headerMesh.position.x + header.width + 8, top: -headerMesh.position.y - 8, bottom: -headerMesh.position.y + header.height + 8 };
    const area = (box, left, top) => Math.max(0, Math.min(box.right, left + card.width) - Math.max(box.left, left)) * Math.max(0, Math.min(box.bottom, top + card.height) - Math.max(box.top, top));
    const cost = ([left, top]) => {
      const off = Math.max(0, 8 - left) + Math.max(0, left + card.width - width + 8) + Math.max(0, 8 - top) + Math.max(0, top + card.height - height + 8);
      const signCost = signStart !== null ? area(signBox, left, top) * 2 : 0;
      return area(lanternBox, left, top) * 2 + area(wordsBox, left, top) * 3 + area(place.facts, left, top) * 3 + area(headerBox, left, top) * 2 + signCost + off * card.height * 4;
    };
    let [x, y] = spots.reduce((best, spot) => (cost(spot) < cost(best) ? spot : best));
    x = THREE.MathUtils.clamp(x, 10, Math.max(10, width - card.width - 10));
    y = THREE.MathUtils.clamp(y, 10, Math.max(10, height - card.height - 10));
    card.mesh.position.set(x, -y, 0);
    card.mesh.scale.set(card.width, card.height, 1);
    // The leader runs from the light to the card's nearest edge.
    const ly = sy + age * 10;
    let cornerX = sx;
    let cornerY = ly;
    if (sx < x) cornerX = x;
    else if (sx > x + card.width) cornerX = x + card.width;
    if (cornerX === sx) cornerY = ly > y + card.height ? y + card.height : y;
    else cornerY = THREE.MathUtils.clamp(ly, y, y + card.height);
    const inside = sx >= x && sx <= x + card.width && ly >= y && ly <= y + card.height;
    card.leader.visible = !inside;
    const positions = card.leader.geometry.attributes.position;
    positions.setXYZ(0, sx, -ly, 0);
    positions.setXYZ(1, cornerX, -cornerY, 0);
    positions.needsUpdate = true;
  }

  // The lights that get a card: the right wall's near ones, the neon and the dark one.
  const cardSlots = [];
  for (let k = 1; k < 8; k++) {
    const kind = slotKind(k, 1);
    if (kind >= 0) cardSlots.push({ z: HALL.first - k * HALL.spacing, position: slotPosition(k, 1, kind), key: `k${kind}`, record: KINDS[kind], was: 0 });
  }
  cardSlots.push({ z: neonZ, position: neonPosition.clone().add(new THREE.Vector3(0, 0.7, 0)), key: 'neon', record: NEON, was: 0 });
  const darkPosition = new THREE.Vector3(lighting.uDark.value.x, lighting.uDark.value.y, lighting.uDark.value.z);
  cardSlots.push({ z: darkPosition.z, position: darkPosition.clone(), key: 'dark', record: DARK, was: 0 });

  // Post-processing.
  const quad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2));
  quad.frustumCulled = false;
  const postScene = new THREE.Scene();
  postScene.add(quad);
  const postCamera = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
  const brightMaterial = fullscreenMaterial(BRIGHT_FRAGMENT, { tInput: { value: sceneTarget.texture }, uThreshold: { value: 0.75 } });
  const blurMaterial = fullscreenMaterial(BLUR_FRAGMENT, { tInput: { value: null }, uDirection: { value: new THREE.Vector2() } });
  const copyMaterial = fullscreenMaterial(COPY_FRAGMENT, { tInput: { value: null } });
  const compositeMaterial = fullscreenMaterial(COMPOSITE_FRAGMENT, {
    tScene: { value: sceneTarget.texture },
    tBloomNear: { value: bloomTargets[0].texture },
    tBloomFar: { value: bloomTargets[2].texture },
    uTime: { value: 0 },
    uText: { value: new THREE.Vector4() },
    uFacts: { value: new THREE.Vector4() },
    uResolution: { value: new THREE.Vector2(1, 1) },
    uBackground: { value: background.clone().convertLinearToSRGB() },
    uExposure: { value: 1.1 },
  });
  function pass(material, target) {
    quad.material = material;
    renderer.setRenderTarget(target);
    renderer.render(postScene, postCamera);
  }
  function blur(target, scratch, radius) {
    blurMaterial.uniforms.tInput.value = target.texture;
    blurMaterial.uniforms.uDirection.value.set(radius / target.width, 0);
    pass(blurMaterial, scratch);
    blurMaterial.uniforms.tInput.value = scratch.texture;
    blurMaterial.uniforms.uDirection.value.set(0, radius / target.height);
    pass(blurMaterial, target);
  }

  // Layout: the lantern hangs where the text leaves room, and the hall's vanishing point is just
  // below it, whatever the hero's shape.
  let width = 1;
  let height = 1;
  let place = placement(root);
  let focal = 600;
  let lanternScale = 1;
  const lanternHome = new THREE.Vector3();
  const vanishing = new THREE.Vector2();
  function layout() {
    const rect = root.getBoundingClientRect();
    width = Math.max(1, Math.round(rect.width));
    height = Math.max(1, Math.round(rect.height));
    const dpr = Math.min(window.devicePixelRatio || 1, small ? 1.5 : 1.75) * quality.level;
    renderer.setPixelRatio(dpr);
    renderer.setSize(width, height, false);
    const w = Math.max(1, Math.floor(width * dpr));
    const h = Math.max(1, Math.floor(height * dpr));
    sceneTarget.setSize(w, h);
    bloomTargets[0].setSize(Math.max(1, w >> 2), Math.max(1, h >> 2));
    bloomTargets[1].setSize(Math.max(1, w >> 2), Math.max(1, h >> 2));
    bloomTargets[2].setSize(Math.max(1, w >> 3), Math.max(1, h >> 3));
    bloomTargets[3].setSize(Math.max(1, w >> 3), Math.max(1, h >> 3));

    place = placement(root);
    // The lantern takes about 0.6 of the space it has; the pixels per unit at unit depth follow.
    focal = Math.max(120, 0.5 * place.size * -LANTERN.z / LANTERN.height);
    vanishing.set(place.x, place.y + 0.3 * place.size);
    const lanternY = (vanishing.y - (place.y - 0.08 * place.size)) * -LANTERN.z / focal;
    lanternHome.set(CAMERA_X, lanternY, LANTERN.z);
    lanternScale = 1;
    camera.fov = THREE.MathUtils.radToDeg(2 * Math.atan(height / 2 / focal));
    camera.aspect = width / height;
    camera.setViewOffset(width, height, width / 2 - vanishing.x, height / 2 - vanishing.y, width, height);
    camera.updateProjectionMatrix();
    flameUniforms.uPixel.value = focal * dpr;
    compositeMaterial.uniforms.uResolution.value.set(width, height);
    const pad = 18;
    const toUv = (box, margin, target) => target.set((box.left - margin) / width, 1 - (box.bottom + margin) / height, (box.right + margin) / width, 1 - (box.top - margin) / height);
    toUv(place.words, pad, compositeMaterial.uniforms.uText.value);
    toUv(place.facts, 0, compositeMaterial.uniforms.uFacts.value);
  }

  // The pointer: pointing down the hall moves the sheet there; the lantern turns to it; the camera
  // leans a little. Taps on the neon are listened to.
  const pointer = new THREE.Vector2();
  const pointerPixel = new THREE.Vector2(-1, -1);
  let pointing = false;
  let lastPointer = -99;
  let scrubTarget = null;
  const raycaster = new THREE.Raycaster();
  const floorPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -HALL.floor);
  const hit = new THREE.Vector3();
  function overWords(x, y) {
    const inside = (b, m) => x > b.left - m && x < b.right + m && y > b.top - m && y < b.bottom + m;
    return inside(place.words, 12) || inside(place.facts, 4);
  }
  function onPointer(event) {
    const rect = root.getBoundingClientRect();
    pointerPixel.set(event.clientX - rect.left, event.clientY - rect.top);
    pointer.set(pointerPixel.x / rect.width * 2 - 1, -(pointerPixel.y / rect.height * 2 - 1));
    if (event.pointerType !== 'mouse' || overWords(pointerPixel.x, pointerPixel.y)) {
      scrubTarget = null;
      return;
    }
    pointing = true;
    lastPointer = time;
    raycaster.setFromCamera(pointer, camera);
    if (raycaster.ray.intersectPlane(floorPlane, hit) && hit.z < 0) scrubTarget = THREE.MathUtils.clamp(hit.z, SHEET.visible, SHEET.start);
    else scrubTarget = SHEET.visible;
  }
  function onLeave() {
    pointing = false;
    scrubTarget = null;
  }

  // The neon's rhythm. Only its own taps count, and only in its stutter: short, short, long, short.
  const taps = [];
  const neonScreen = [new THREE.Vector3(), new THREE.Vector3()];
  function toScreen(point, out) {
    out.copy(point).project(camera);
    out.set((out.x + 1) / 2 * width, (1 - out.y) / 2 * height, out.z);
    return out;
  }
  function onNeon(x, y) {
    toScreen(neonPosition.clone().add(new THREE.Vector3(0, 0.65, 0)), neonScreen[0]);
    toScreen(neonPosition.clone().add(new THREE.Vector3(0, -0.65, 0)), neonScreen[1]);
    const a = neonScreen[0];
    const b = neonScreen[1];
    const abx = b.x - a.x;
    const aby = b.y - a.y;
    const t = THREE.MathUtils.clamp(((x - a.x) * abx + (y - a.y) * aby) / Math.max(1e-6, abx * abx + aby * aby), 0, 1);
    const dx = a.x + abx * t - x;
    const dy = a.y + aby * t - y;
    return Math.hypot(dx, dy) < Math.max(26, Math.abs(aby) * 0.12);
  }
  // Five taps whose gaps, measured in the phrase's own beat, are short, short, long, short, each
  // within a quarter of a beat. Any tempo from a brisk to a slow one will do.
  function rhythm(times) {
    if (times.length < 5) return null;
    const recent = times.slice(-5);
    const gaps = recent.slice(1).map((t, i) => t - recent[i]);
    const beat = gaps.reduce((sum, gap) => sum + gap, 0) / STUTTER.reduce((sum, beats) => sum + beats, 0);
    if (beat < 0.18 || beat > 0.8) return null;
    return gaps.every((gap, i) => Math.abs(gap - STUTTER[i] * beat) <= 0.24 * beat) ? beat : null;
  }
  function onPress(event) {
    const rect = root.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    if (!onNeon(x, y) || overWords(x, y)) {
      taps.length = 0;
      return;
    }
    const now = performance.now() / 1000;
    if (taps.length && now - taps[taps.length - 1] > 2.2) taps.length = 0;
    taps.push(now);
    while (taps.length > 5) taps.shift();
    const beat = rhythm(taps);
    if (beat !== null && signStart === null) {
      taps.length = 0;
      signUp(beat);
    }
  }
  function signUp(beat) {
    signStart = time;
    signBeat = Math.max(0.36, beat);
    spawnCard(neonPosition.clone().add(new THREE.Vector3(0, 0.8, 0)), 'neon-sign', NEON,
      [['flags ', 'text'], ['DYNAMIC', 'old'], [' -> ', 'arrow'], ['DYNAMIC | SIGNAGE', 'new']], 5);
  }
  const hero = root.closest('.dw-hero') || root;
  if (!reduceMotion) {
    hero.addEventListener('pointermove', onPointer, { passive: true });
    hero.addEventListener('pointerdown', onPress, { passive: true });
    hero.addEventListener('pointerleave', onLeave, { passive: true });
  }

  // The sheet's round: rest at the lantern, run down the hall, race to the last light, hold, then
  // the load order changes and it runs back.
  const ROUND = { rest: 2.4, sweep: 8.0, race: 1.8, hold: 7.0, rewind: 2.2 };
  const roundLength = ROUND.rest + ROUND.sweep + ROUND.race + ROUND.hold + ROUND.rewind;
  function autoSheet(t) {
    let u = t % roundLength;
    if (u < ROUND.rest) return SHEET.start;
    u -= ROUND.rest;
    if (u < ROUND.sweep) return THREE.MathUtils.lerp(SHEET.start, SHEET.visible, easeInOut(u / ROUND.sweep));
    u -= ROUND.sweep;
    if (u < ROUND.race) {
      const f = u / ROUND.race;
      return SHEET.visible * Math.pow(SHEET.end / SHEET.visible, f * f);
    }
    u -= ROUND.race;
    if (u < ROUND.hold) return SHEET.end;
    u -= ROUND.hold;
    const f = easeInOut(u / ROUND.rewind);
    return SHEET.end * Math.pow(SHEET.start / SHEET.end, f);
  }
  function changedLights(z) {
    const passed = Math.floor((HALL.first - z - 0.35) / HALL.spacing) + 1;
    return 2 * THREE.MathUtils.clamp(passed, 0, HALL.perSide);
  }

  // The loop.
  const clock = new THREE.Clock();
  let time = 0;
  let roundTime = reduceMotion ? ROUND.rest + ROUND.sweep * 0.42 : 0;
  let sheetZ = SHEET.start;
  let visible = false;
  let running = false;
  let first = true;
  let lost = false;
  const neonFlicker = new Flicker(0.1, 7);
  const chance = TEST ? random(1234) : Math.random;
  let nextHint = 14 + chance() * 8;
  let signStart = null;
  let signBeat = STUTTER_BEAT;
  const lean = new THREE.Vector2();
  const tmp = new THREE.Vector3();
  let lanternAngle = 0.4;

  // The stutter: the neon dips at each tap of the pattern.
  function stutterDip(age, beat) {
    let t = 0;
    let dip = 1;
    for (let i = 0; i <= STUTTER.length; i++) {
      const d = age - t;
      if (d > -0.02 && d < 0.14) dip = Math.min(dip, 0.22);
      if (i < STUTTER.length) t += STUTTER[i] * beat;
    }
    return dip;
  }

  function frame() {
    running = false;
    if (lost) return;
    const rawDt = clock.getDelta();
    if (TEST && !reduceMotion) {
      if (time < TEST.at) {
        while (time < TEST.at - 1e-9) {
          if (TEST.sign !== null && signStart === null && time >= TEST.sign) signUp(STUTTER_BEAT);
          step(Math.min(1 / 60, TEST.at - time));
        }
      }
      draw();
      root.dataset.s3lfFrames = String(Number(root.dataset.s3lfFrames || 0) + 1);
      if (visible && !document.hidden) requestFrame();
      return;
    }
    const dt = Math.min(rawDt, 0.05);
    if (!reduceMotion && rawDt < 0.5) {
      quality.slow = rawDt > 1 / 40 ? quality.slow + rawDt : Math.max(0, quality.slow - rawDt * 0.5);
      if (quality.slow > 1.5 && quality.level > 0.5) {
        quality.level = Math.max(0.5, quality.level - 0.15);
        quality.slow = 0;
        layout();
      }
    }
    step(dt);
    draw();
    if (visible && !reduceMotion && !document.hidden) requestFrame();
  }

  function step(dt) {
    if (!reduceMotion) time += dt;
    shared.uTime.value = time;
    compositeMaterial.uniforms.uTime.value = time;

    // The sheet: the pointer's depth while it points down the hall, the round otherwise.
    const scrubbing = scrubTarget !== null && pointing && time - lastPointer < 3;
    if (!reduceMotion) {
      if (scrubbing) {
        sheetZ += (scrubTarget - sheetZ) * Math.min(1, dt * 5);
        // Resume the round from here when the pointer lets go.
        const sweepProgress = THREE.MathUtils.clamp((sheetZ - SHEET.start) / (SHEET.visible - SHEET.start), 0, 1);
        roundTime = ROUND.rest + ROUND.sweep * sweepProgress * 0.5;
      } else {
        roundTime += dt;
        const target = autoSheet(roundTime);
        sheetZ += (target - sheetZ) * Math.min(1, dt * 6);
        if (Math.abs(target - sheetZ) > 60) sheetZ = target;
      }
    } else {
      sheetZ = autoSheet(roundTime);
    }
    shared.uSheet.value = sheetZ;
    sheet.position.set(0, (HALL.spring + HALL.half + HALL.floor) / 2, sheetZ);
    const sheetVisible = sheetZ < SHEET.start + 0.05 && sheetZ > SHEET.end + 1;
    sheetUniforms.uOpacity.value = sheetVisible ? 1 - smooth(LANTERN.z - 2.5, LANTERN.z - 0.4, sheetZ) : 0;
    lighting.uSheetOn.value = sheetVisible ? 1 - smooth(SHEET.start - 0.9, SHEET.start - 0.1, sheetZ) : 0;
    setCount(changedLights(sheetZ));

    // The neon: OpenMW's fast flicker until fixed, its stutter now and then, and the sign.
    const neonFix = smooth(0, 0.7, neonZ - sheetZ);
    const flick = reduceMotion ? 0.8 : neonFlicker.update(dt);
    let hint = 1;
    if (!reduceMotion && time > nextHint) {
      const age = time - nextHint;
      const span = STUTTER.reduce((a, b) => a + b, 0) * STUTTER_BEAT + 0.3;
      hint = stutterDip(age, STUTTER_BEAT);
      if (age > span) nextHint = time + 22 + chance() * 12;
    }
    let signAmount = 0;
    let pulse = 1;
    if (signStart !== null) {
      const age = time - signStart;
      const drawIn = 3.2;
      const hold = 9;
      const drawOut = 1.6;
      signAmount = age < drawIn ? age / drawIn : age < drawIn + hold ? 1 : Math.max(0, 1 - (age - drawIn - hold) / drawOut);
      const cycle = STUTTER.reduce((a, b) => a + b, 0) * signBeat + signBeat * 2;
      if (age > drawIn * 0.5 && age < drawIn + hold) pulse = 0.72 + 0.28 * (1 - (1 - stutterDip(age % cycle, signBeat)) / 0.78);
      if (age > drawIn + hold + drawOut) signStart = null;
      // The strokes draw in order, then undraw in reverse.
      const total = strokes.length;
      strokes.forEach((stroke, i) => {
        const start = i / total;
        const local = THREE.MathUtils.clamp((signAmount - start) * total, 0, 1);
        stroke.mesh.visible = local > 0.001;
        stroke.mesh.geometry.setDrawRange(0, Math.floor(stroke.count * local / 3) * 3);
        stroke.uniforms.uIntensity.value = 2.4 * (0.85 + 0.15 * Math.sin(time * 3.1 + i));
      });
    } else {
      for (const stroke of strokes) stroke.mesh.visible = false;
    }
    shared.uPulse.value = pulse;
    const neonBright = THREE.MathUtils.lerp(flick, 1, neonFix) * hint * pulse;
    const neonColor = linear(NEON.vanilla).multiplyScalar(neonBright).lerp(linear(NEON.fixed).multiplyScalar(FIXED_GAIN * hint * pulse), neonFix);
    lighting.uNeonColor.value.copy(neonColor);
    lighting.uNeon.value.w = THREE.MathUtils.lerp(NEON.radius[0], NEON.radius[1], neonFix) / 128;
    tubeUniforms.uColor.value.copy(neonColor).multiplyScalar(1 / Math.max(0.05, neonBright * 0.8));
    tubeUniforms.uIntensity.value = 0.4 + neonBright * 1.4;
    haloUniforms.uColor.value.copy(neonColor).multiplyScalar(1.4);
    neonHalo.quaternion.copy(camera.quaternion);
    lighting.uSign.value.set(sign.position.x, sign.position.y + 0.1, sign.position.z, signAmount * 1.2);
    lighting.uSignDark.value.set(sign.position.x, sign.position.y - 1.24 * sign.scale.y, sign.position.z + 0.2, signAmount);

    // The negative light shrinks to nothing as the sheet reaches it.
    const darkFix = smooth(0, 0.7, darkPosition.z - sheetZ);
    lighting.uDark.value.w = THREE.MathUtils.lerp(DARK.radius[0], DARK.radius[1], darkFix) / 128;
    voidUniforms.uAmount.value = 1 - darkFix;
    voidMesh.visible = darkFix < 0.999;
    voidMesh.quaternion.copy(camera.quaternion);
    voidMesh.scale.setScalar(0.4 + 0.6 * (1 - darkFix));

    // Log cards, for each light the sheet has just passed going down the hall.
    for (const slot of cardSlots) {
      const fixNow = smooth(0, 0.7, slot.z - sheetZ);
      if (!reduceMotion && fixNow >= 0.5 && slot.was < 0.5) spawnCard(slot.position, slot.key, slot.record);
      slot.was = fixNow;
    }
    if (place.above) {
      headerMesh.position.set(12, -12, 0);
    } else {
      const headerRight = Math.min(width - 12, place.x + place.size * 0.62);
      const headerBottom = Math.min(height - 12, place.y + place.size * 0.5);
      headerMesh.position.set(Math.max(10, headerRight - header.width), -(headerBottom - header.height), 0);
    }
    headerMesh.scale.set(header.width, header.height, 1);
    headerMesh.material.uniforms.uOpacity.value = 0.9;
    for (let i = cards.length - 1; i >= 0; i--) {
      const card = cards[i];
      const age = (time - card.born) / card.life;
      if (age >= 1) {
        overlayScene.remove(card.mesh, card.leader);
        card.mesh.material.dispose();
        card.leader.material.dispose();
        card.leader.geometry.dispose();
        cards.splice(i, 1);
        continue;
      }
      placeCard(card, time - card.born);
      const opacity = smooth(0, 0.1, age) * (1 - smooth(0.8, 1, age));
      card.mesh.material.uniforms.uOpacity.value = opacity;
      card.leader.material.uniforms.uOpacity.value = opacity * 0.7;
    }

    // The camera leans with the pointer; the lantern turns, sways on its chain, and faces the pointer.
    const follow = reduceMotion ? 1 : Math.min(1, dt * 2);
    const wanted = pointing && time - lastPointer < 4 ? pointer : new THREE.Vector2(Math.sin(time * 0.13) * 0.25, Math.sin(time * 0.09) * 0.15);
    lean.x += (wanted.x - lean.x) * follow;
    lean.y += (wanted.y - lean.y) * follow;
    camera.position.set(CAMERA_X + lean.x * 0.14, lean.y * 0.08, 0);
    camera.updateMatrixWorld();
    lanternAngle += dt * (0.32 + (pointing ? lean.x * 0.25 : 0));
    const sway = Math.sin(time * 0.6) * 0.035;
    lantern.position.copy(lanternHome).add(new THREE.Vector3(Math.sin(time * 0.6) * 0.04, Math.cos(time * 1.2) * 0.012, 0));
    lantern.rotation.set(Math.sin(time * 0.47) * 0.03, lanternAngle, sway);
    lantern.scale.setScalar(lanternScale);
    lantern.updateMatrixWorld();
    const top = new THREE.Vector3(0, 0.73, 0).applyMatrix4(lantern.matrixWorld);
    const anchor = new THREE.Vector3(lanternHome.x, HALL.spring + HALL.half - 0.05, LANTERN.z);
    chain.position.copy(top).add(anchor).multiplyScalar(0.5);
    chain.scale.set(1, Math.max(0.01, anchor.distanceTo(top)), 1);
    chain.lookAt(anchor);
    chain.rotateX(Math.PI / 2);
    core.position.copy(lantern.position).add(new THREE.Vector3(0, -0.02, 0));
    core.quaternion.copy(camera.quaternion);
    const flameFlicker = 0.92 + 0.08 * Math.sin(time * 5.3) * Math.sin(time * 3.7);
    core.material.uniforms.uGain.value = flameFlicker;
    for (const pane of panes) pane.material.uniforms.uFlame.value = flameFlicker;
    lighting.uLanternPos.value.copy(lantern.position);
    beams.forEach((beam, i) => {
      const a = lanternAngle + i * Math.PI / 2;
      beam.material.uniforms.uAxis.value.set(Math.sin(a), -0.2, Math.cos(a)).normalize();
      beam.material.uniforms.uApex.value.copy(lantern.position);
    });
    lighting.uLanternAngle.value = lanternAngle;
    lighting.uFlame.value.set(lantern.position.x, lantern.position.y, lantern.position.z, 2.8);
    dustUniforms.uFlamePos.value.copy(lantern.position);
  }

  function draw() {
    renderer.setRenderTarget(sceneTarget);
    renderer.setClearColor(fog, 1);
    renderer.clear();
    renderer.render(scene, camera);

    pass(brightMaterial, bloomTargets[0]);
    blur(bloomTargets[0], bloomTargets[1], 1.0);
    blur(bloomTargets[0], bloomTargets[1], 2.0);
    copyMaterial.uniforms.tInput.value = bloomTargets[0].texture;
    pass(copyMaterial, bloomTargets[2]);
    blur(bloomTargets[2], bloomTargets[3], 1.5);
    blur(bloomTargets[2], bloomTargets[3], 3.0);
    pass(compositeMaterial, null);
    overlayCamera.right = width;
    overlayCamera.bottom = -height;
    overlayCamera.updateProjectionMatrix();
    renderer.render(overlayScene, overlayCamera);

    if (first) {
      first = false;
      root.classList.add('is-live');
    }
  }

  function requestFrame() {
    if (running || lost) return;
    running = true;
    requestAnimationFrame(frame);
  }

  canvas.addEventListener('webglcontextlost', (event) => {
    event.preventDefault();
    lost = true;
    root.classList.remove('is-live');
  });
  canvas.addEventListener('webglcontextrestored', () => {
    canvas.remove();
    root.classList.remove('is-live');
    mount(root);
  });

  layout();
  new ResizeObserver(() => {
    layout();
    requestFrame();
  }).observe(root);
  if (document.fonts) {
    document.fonts.ready.then(() => {
      layout();
      requestFrame();
    });
  }
  new IntersectionObserver((entries) => {
    visible = entries.some((entry) => entry.isIntersecting);
    if (visible) {
      clock.getDelta();
      requestFrame();
    }
  }).observe(root);
  document.addEventListener('visibilitychange', () => {
    if (!document.hidden && visible) {
      clock.getDelta();
      requestFrame();
    }
  });
}

for (const root of document.querySelectorAll('[data-dw-hero-art]')) mount(root);
