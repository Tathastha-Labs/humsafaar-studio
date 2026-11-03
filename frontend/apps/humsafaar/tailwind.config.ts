import type { Config } from "tailwindcss";
import colors from "tailwindcss/colors";

export default {
  content: [
    "./app/index.html",
    "./app/src/**/*.{js,jsx,ts,tsx}",
    "./app/src/*.{js,jsx,ts,tsx}",
    "../../libs/components/**/*.{js,jsx,ts,tsx}",
  ],
  theme: {
    extend: {
      fontFamily: {
        "custom-font": ["Fira Sans", "sans-serif"],
        // Brutalist type system, matching humsafaar-webapp.
        mono: [
          "Geist Mono",
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Consolas",
          "monospace",
        ],
        display: [
          "Archivo",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Roboto",
          "Helvetica Neue",
          "Arial",
          "sans-serif",
        ],
        sans: [
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Roboto",
          "Helvetica Neue",
          "Arial",
          "sans-serif",
        ],
      },
      keyframes: {
        fadeIn: {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
        shimmer: {
          "0%": { backgroundPosition: "200% 0" },
          "100%": { backgroundPosition: "-200% 0" },
        },
        marquee: {
          "0%": { transform: "translateX(0%)" },
          "100%": { transform: "translateX(-100%)" },
        },
      },
      animation: {
        fadeIn: "fadeIn 0.2s ease-in-out",
        shimmer: "shimmer 2s ease-in-out infinite",
        marquee: "marquee 8s linear infinite paused",
      },
    },
    colors: {
      transparent: "transparent",
      current: "currentColor",
      // White follows the theme (Tathastha is light, so "white" text and tints become black ones).
      white: "rgb(var(--st-white-rgb, 255 255 255) / <alpha-value>)",
      gray: colors.gray,
      black: colors.black,
      blue: { ...colors.blue, DEFAULT: "#2E70FF" },
      red: { ...colors.red, DEFAULT: "#D33242" },
      green: { ...colors.green, DEFAULT: "#308752" },
      yellow: colors.yellow,
      purple: colors.purple,
      pink: colors.pink,
      orange: colors.orange,
      amber: colors.amber,
      emerald: colors.emerald,
      teal: colors.teal,
      cyan: colors.cyan,
      sky: colors.sky,
      violet: colors.violet,
      fuchsia: colors.fuchsia,
      rose: colors.rose,
      indigo: colors.indigo,
      lime: colors.lime,
      primary: {
        // Tathastha teal (tathastha.com); 500 is the brand teal.
        DEFAULT: "#39BDC6",
        "50": "#F0FAFB",
        "100": "#D6F2F4",
        "200": "#ADE5E9",
        "300": "#84D7DD",
        "400": "#5CCBD2",
        "500": "#39BDC6",
        "600": "#2BA0A8",
        "700": "#1F8A92",
        "800": "#0C4B4F",
        "900": "#081F21",
        wrapper: "#39394d",
      },
      ui: {
        // Brutalist system (matches humsafaar-webapp): near-black surfaces
        // with white/15 hairline borders. Values come from the --st-* theme
        // variables in app/src/styles/base.css.
        background: "var(--st-bg)",
        panel: "var(--st-panel)",
        "panel-border": "var(--st-panel-border)",
        border: "var(--st-panel-border)",
        controls: "rgb(var(--st-controls-rgb) / <alpha-value>)",
        "controls-button": "rgb(var(--st-controls-rgb) / <alpha-value>)",
        "controls-border": "var(--st-controls-border)",
        divider: "var(--st-divider)",
        ink: "rgb(var(--st-fg-rgb) / <alpha-value>)",
        "accent-ink": "var(--st-accent-ink, #74aaff)",
        sunken: "var(--st-sunken)",
      },
      "base-fg": "rgb(var(--st-fg-rgb) / <alpha-value>)",
      media: {
        "audio-tts": "#6AAA88",
        "audio-v2v": "#6691B9",
        "audio-upload": "#9760C2",
        "audio-demo": "#B96675",
        "is-new": "#FFDC26",
      },
      inference: {
        pending: "#6b728b",
        generating: "#46527C",
        error: "#924C4B",
      },
      "axis-x": "#D33242",
      "axis-y": "#308752",
      "axis-z": "#2E70FF",

      // shadcn tokens consumed by @storyteller/ui-video-editor. CSS
      // variables live in app/src/styles/base.css. primary/secondary
      // are intentionally NOT mapped here — the Tauri app already has
      // its own brand colors elsewhere in this palette and the editor
      // chrome inherits them via the --st-* surfaces.
      background: "hsl(var(--background))",
      foreground: "hsl(var(--foreground))",
      card: {
        DEFAULT: "hsl(var(--card))",
        foreground: "hsl(var(--card-foreground))",
      },
      popover: {
        DEFAULT: "hsl(var(--popover))",
        foreground: "hsl(var(--popover-foreground))",
      },
      muted: {
        DEFAULT: "hsl(var(--muted))",
        foreground: "hsl(var(--muted-foreground))",
      },
      accent: {
        DEFAULT: "hsl(var(--accent))",
        foreground: "hsl(var(--accent-foreground))",
      },
      destructive: {
        DEFAULT: "hsl(var(--destructive))",
        foreground: "hsl(var(--destructive-foreground))",
      },
      border: "hsl(var(--border))",
      input: "hsl(var(--input))",
      ring: "hsl(var(--ring))",
      facebook: "#4267b2",
      reddit: "#ff5700",
      whatsapp: "#25d366",
      x: "#000000",
      email: "#858585",
    },
  },
  plugins: [],
  safelist: [
    // lucide-react adds this class at runtime; keep the base-layer sizing rule
    // in styles/tailwind.css from being purged.
    "lucide",
    "bg-character-selected",
    "bg-character-unselected",
    "bg-character-clip",
    "bg-camera-selected",
    "bg-camera-unselected",
    "bg-camera-clip",
    "bg-global_audio-selected",
    "bg-global_audio-unselected",
    "bg-global_audio-clip",
    "bg-object-selected",
    "bg-object-unselected",
    "bg-object-clip",
    "bg-prompt-selected",
    "bg-prompt-unselected",
    "bg-prompt-clip",
    "bg-facebook",
    "bg-reddit",
    "bg-whatsapp",
    "bg-x",
    "bg-email",
  ],
} satisfies Config;
