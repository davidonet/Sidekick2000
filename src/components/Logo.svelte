<svg
  width="290"
  height="96"
  viewBox="0 0 290 96"
  xmlns="http://www.w3.org/2000/svg"
  role="img"
  aria-label="Sidekick2000"
>
  <defs>
    <!-- Chrome gradient: bright top → vivid mid-blue → strong blue (visible on dark bg) -->
    <linearGradient id="sk-chrome" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%"   stop-color="#ffffff"/>
      <stop offset="12%"  stop-color="#d0eeff"/>
      <stop offset="30%"  stop-color="#70c0ff"/>
      <stop offset="55%"  stop-color="#1878e8"/>
      <stop offset="78%"  stop-color="#2060d0"/>
      <stop offset="100%" stop-color="#4888ff"/>
    </linearGradient>
    <!-- Extrusion: vivid blue offset — visible against dark backgrounds -->
    <linearGradient id="sk-extrude" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%"   stop-color="#1040a0"/>
      <stop offset="100%" stop-color="#050e30"/>
    </linearGradient>
    <!-- Hat crown: warm amber/brown -->
    <linearGradient id="sk-hat-crown" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%"   stop-color="#d07828"/>
      <stop offset="55%"  stop-color="#8a4412"/>
      <stop offset="100%" stop-color="#4e2208"/>
    </linearGradient>
    <!-- Hat brim -->
    <linearGradient id="sk-hat-brim" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%"   stop-color="#a85820"/>
      <stop offset="100%" stop-color="#3e1a00"/>
    </linearGradient>
    <!-- Gold "2000" -->
    <linearGradient id="sk-gold" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%"   stop-color="#fff0a0"/>
      <stop offset="45%"  stop-color="#f8c800"/>
      <stop offset="100%" stop-color="#b88000"/>
    </linearGradient>
    <!-- Glow behind chrome text -->
    <filter id="sk-glow" x="-8%" y="-15%" width="116%" height="140%">
      <feGaussianBlur stdDeviation="2.5" result="blur"/>
      <feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
    <!-- Soft blue ambient glow under the text block -->
    <filter id="sk-ambient" x="-5%" y="-20%" width="110%" height="150%">
      <feGaussianBlur stdDeviation="4" result="b"/>
      <feFlood flood-color="#2060ff" flood-opacity="0.25" result="c"/>
      <feComposite in="c" in2="b" operator="in" result="g"/>
      <feMerge><feMergeNode in="g"/><feMergeNode in="SourceGraphic"/></feMerge>
    </filter>
  </defs>

  <!-- ── Cowboy hat ──────────────────────────────────────────── -->
  <ellipse cx="32" cy="33" rx="30" ry="6.5" fill="#00000066"/>
  <ellipse cx="32" cy="31" rx="30" ry="6.5" fill="url(#sk-hat-brim)" stroke="#1a0800" stroke-width="0.8"/>
  <path d="M10 31 C10 31 10 28 12 18 C14 8 20 2 32 2 C44 2 50 8 52 18 C54 28 54 31 54 31 Z"
        fill="url(#sk-hat-crown)" stroke="#1a0800" stroke-width="0.8"/>
  <!-- Crown crease -->
  <path d="M19 7 Q32 14 45 7" fill="none" stroke="#2a1000" stroke-width="1.5" stroke-linecap="round" opacity="0.7"/>
  <!-- Hat band -->
  <path d="M10 31 C10 31 10 26 12 24 L52 24 C54 26 54 31 54 31 Z" fill="#180600" opacity="0.9"/>
  <!-- Band highlight line -->
  <path d="M13 24 L51 24" fill="none" stroke="#e07038" stroke-width="0.9" opacity="0.6"/>
  <!-- Crown sheen -->
  <path d="M16 11 Q32 7 48 11" fill="none" stroke="#f0a860" stroke-width="1.2" stroke-linecap="round" opacity="0.5"/>

  <!-- ── SIDEKICK — 3D extruded chrome text ──────────────────── -->
  <!-- Ambient glow layer -->
  <text x="2" y="74" font-family="Impact, 'Arial Black', sans-serif" font-size="56"
        fill="url(#sk-chrome)" letter-spacing="1" filter="url(#sk-ambient)" opacity="0.4">SIDEKICK</text>
  <!-- Extrusion layers (step down-right) -->
  <text x="8" y="80" font-family="Impact, 'Arial Black', sans-serif" font-size="56"
        fill="url(#sk-extrude)" letter-spacing="1" opacity="0.5">SIDEKICK</text>
  <text x="7" y="79" font-family="Impact, 'Arial Black', sans-serif" font-size="56"
        fill="url(#sk-extrude)" letter-spacing="1" opacity="0.6">SIDEKICK</text>
  <text x="6" y="78" font-family="Impact, 'Arial Black', sans-serif" font-size="56"
        fill="url(#sk-extrude)" letter-spacing="1" opacity="0.75">SIDEKICK</text>
  <text x="5" y="77" font-family="Impact, 'Arial Black', sans-serif" font-size="56"
        fill="url(#sk-extrude)" letter-spacing="1" opacity="0.88">SIDEKICK</text>
  <text x="4" y="76" font-family="Impact, 'Arial Black', sans-serif" font-size="56"
        fill="url(#sk-extrude)" letter-spacing="1">SIDEKICK</text>
  <!-- Chrome face with thin outline for crispness -->
  <text x="2" y="74" font-family="Impact, 'Arial Black', sans-serif" font-size="56"
        fill="url(#sk-chrome)" letter-spacing="1"
        stroke="#1050b0" stroke-width="0.5" paint-order="stroke"
        filter="url(#sk-glow)">SIDEKICK</text>
  <!-- Reflection streak -->
  <rect x="2" y="35" width="262" height="13" fill="white" opacity="0.10" transform="skewX(-12)"/>

  <!-- ── TM mark ─────────────────────────────────────────────── -->
  <text x="268" y="30" font-family="Arial, sans-serif" font-size="10" font-weight="bold"
        fill="#80b8f0" letter-spacing="0.5">TM</text>

  <!-- ── "2000" subtitle ─────────────────────────────────────── -->
  <text x="136" y="93" font-family="'Courier New', Courier, monospace" font-size="13"
        fill="url(#sk-gold)" letter-spacing="5" text-anchor="middle" font-weight="bold">2 0 0 0</text>
</svg>
