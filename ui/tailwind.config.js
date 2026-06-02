/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./popup.html", "./src/**/*.{ts,tsx}"],
  darkMode: "media",
  theme: {
    extend: {
      colors: {
        bg: { DEFAULT: "rgb(var(--bg) / <alpha-value>)", subtle: "rgb(var(--bg-subtle) / <alpha-value>)" },
        fg: { DEFAULT: "rgb(var(--fg) / <alpha-value>)", subtle: "rgb(var(--fg-subtle) / <alpha-value>)" },
        accent: { DEFAULT: "rgb(var(--accent) / <alpha-value>)" },
        border: { DEFAULT: "rgb(var(--border) / <alpha-value>)" },
      },
      fontFamily: {
        sans: [
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "PingFang SC",
          "Hiragino Sans GB",
          "Microsoft YaHei",
          "sans-serif",
        ],
        mono: ["ui-monospace", "SFMono-Regular", "Menlo", "monospace"],
      },
      borderRadius: { xl: "0.875rem" },
    },
  },
  plugins: [],
};
