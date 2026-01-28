import type { Config } from "tailwindcss";

export default {
  content: ["./app/**/{**,.client,.server}/**/*.{js,jsx,ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "-apple-system",
          "BlinkMacSystemFont",
          "SF Pro Display",
          "SF Pro Text",
          "Inter",
          "Segoe UI",
          "Roboto",
          "Helvetica Neue",
          "sans-serif",
        ],
      },
      colors: {
        // Apple-inspired color palette
        apple: {
          blue: "#007AFF",
          green: "#34C759",
          indigo: "#5856D6",
          orange: "#FF9500",
          pink: "#FF2D55",
          purple: "#AF52DE",
          red: "#FF3B30",
          teal: "#5AC8FA",
          yellow: "#FFCC00",
        },
        // Neutral grays (Apple-style)
        gray: {
          50: "#F9FAFB",
          100: "#F2F2F7",
          200: "#E5E5EA",
          300: "#D1D1D6",
          400: "#AEAEB2",
          500: "#8E8E93",
          600: "#636366",
          700: "#48484A",
          800: "#3A3A3C",
          900: "#1C1C1E",
        },
      },
      borderRadius: {
        "apple": "12px",
        "apple-lg": "16px",
        "apple-xl": "20px",
        "apple-2xl": "24px",
      },
      boxShadow: {
        "apple": "0 4px 6px -1px rgba(0, 0, 0, 0.05), 0 2px 4px -1px rgba(0, 0, 0, 0.03)",
        "apple-md": "0 10px 15px -3px rgba(0, 0, 0, 0.08), 0 4px 6px -2px rgba(0, 0, 0, 0.04)",
        "apple-lg": "0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 10px 10px -5px rgba(0, 0, 0, 0.04)",
      },
      backdropBlur: {
        "apple": "20px",
      },
      animation: {
        "fade-in": "fadeIn 0.2s ease-out",
        "slide-up": "slideUp 0.3s ease-out",
        "slide-down": "slideDown 0.3s ease-out",
      },
      keyframes: {
        fadeIn: {
          "0%": { opacity: "0" },
          "100%": { opacity: "1" },
        },
        slideUp: {
          "0%": { transform: "translateY(10px)", opacity: "0" },
          "100%": { transform: "translateY(0)", opacity: "1" },
        },
        slideDown: {
          "0%": { transform: "translateY(-10px)", opacity: "0" },
          "100%": { transform: "translateY(0)", opacity: "1" },
        },
      },
    },
  },
  plugins: [],
} satisfies Config;
