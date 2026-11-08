import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // Les tests qui ont besoin du DOM déclarent `// @vitest-environment jsdom`.
    environment: "node",
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
