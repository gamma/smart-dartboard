import { rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const archive = new URL(
  "../src-tauri/gen/apple/build/sdb-native-m0_iOS.xcarchive",
  import.meta.url,
);

await rm(fileURLToPath(archive), { recursive: true, force: true });
