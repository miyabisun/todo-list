import { rmSync } from "node:fs";

export default function teardown() {
  rmSync(process.env.TODO_E2E_DATA!, { recursive: true, force: true });
}
