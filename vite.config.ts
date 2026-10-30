import { defineConfig } from "vite";
import { resolve } from "path";

export default defineConfig({
  root: "ui",
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: {
    outDir: "dist",
    emptyOutDir: true,
    target: "es2021",
    rollupOptions: {
      input: {
        main: resolve(__dirname, "ui/index.html"),
        search: resolve(__dirname, "ui/search.html"),
        dock: resolve(__dirname, "ui/dock.html"),
        topbar: resolve(__dirname, "ui/topbar.html"),
        switcher: resolve(__dirname, "ui/switcher.html"),
        expose: resolve(__dirname, "ui/expose.html"),
      },
    },
  },
});
