import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { defineConfig } from "@playwright/test";

// The real binary (frontend build embedded) runs on a throwaway data
// directory. Workers re-read this file, so the directory is created once.
process.env.TODO_E2E_DATA ??= mkdtempSync(join(tmpdir(), "todo-list-e2e-"));
const port = 5187;

export default defineConfig({
  testDir: "./e2e",
  workers: 1,
  globalTeardown: "./e2e/teardown.ts",
  use: {
    baseURL: `http://127.0.0.1:${port}`,
    browserName: "chromium",
    locale: "ja-JP",
  },
  webServer: {
    command: "../target/debug/todo-list",
    url: `http://127.0.0.1:${port}/healthz`,
    env: {
      PORT: String(port),
      APP_DATA_DIR: process.env.TODO_E2E_DATA,
      LOG_LEVEL: "warn",
    },
  },
});
