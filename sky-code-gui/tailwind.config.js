/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["class"],
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        // shadcn/ui tokens (HSL-based)
        border: "hsl(var(--border))",
        input: "hsl(var(--input))",
        ring: "hsl(var(--ring))",
        background: "hsl(var(--background))",
        foreground: "hsl(var(--foreground))",
        primary: {
          DEFAULT: "hsl(var(--primary))",
          foreground: "hsl(var(--primary-foreground))",
        },
        secondary: {
          DEFAULT: "hsl(var(--secondary))",
          foreground: "hsl(var(--secondary-foreground))",
        },
        muted: {
          DEFAULT: "hsl(var(--muted))",
          foreground: "hsl(var(--muted-foreground))",
        },
        // SkyCode semantic design tokens (CSS var-based)
        bg: {
          0:   "var(--bg-0)",
          100: "var(--bg-100)",
          200: "var(--bg-200)",
          300: "var(--bg-300)",
        },
        text: {
          100: "var(--text-100)",
          200: "var(--text-200)",
          300: "var(--text-300)",
          400: "var(--text-400)",
          500: "var(--text-500)",
        },
        accent: {
          DEFAULT: "var(--accent)",
          hover:   "var(--accent-hover)",
        },
      },
      borderRadius: {
        lg: "var(--radius)",
        md: "calc(var(--radius) - 2px)",
        sm: "calc(var(--radius) - 4px)",
      },
      fontFamily: {
        sans:  ["Inter", "system-ui", "sans-serif"],
        mono:  ['"JetBrains Mono"', '"Fira Code"', "monospace"],
        serif: ['"Source Serif 4"', "Georgia", "serif"],
      },
      keyframes: {
        fadeIn: {
          from: { opacity: "0", transform: "translateY(6px) scale(0.98)", filter: "blur(3px)" },
          to:   { opacity: "1", transform: "translateY(0) scale(1)",      filter: "blur(0)"   },
        },
      },
      animation: {
        "fade-in": "fadeIn 0.2s ease-out both",
      },
    },
  },
  plugins: [],
};
