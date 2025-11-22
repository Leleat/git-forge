import { resolve } from "node:path";
import { defineConfig } from "vite";

export default defineConfig({
    resolve: {
        alias: {
            "@": resolve(import.meta.dirname, "./src"),
        },
    },
    build: {
        outDir: "dist",
        emptyOutDir: true,
        target: "esnext",
        ssr: true,
        minify: true,
        rollupOptions: {
            input: "src/main.ts",
            output: {
                entryFileNames: "git-forge",
                format: "esm",
                inlineDynamicImports: true,
            },
        },
    },
});
