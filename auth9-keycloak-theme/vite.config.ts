import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { keycloakify } from "keycloakify/vite-plugin";

export default defineConfig({
  plugins: [
    react(),
    keycloakify({
      themeName: "auth9",
      accountThemeImplementation: "none",
      environmentVariables: [
        { name: "AUTH9_API_URL", default: "http://localhost:8080" }
      ]
    })
  ]
});
