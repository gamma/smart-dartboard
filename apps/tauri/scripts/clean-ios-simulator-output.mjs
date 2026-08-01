import { rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const generatedOutputs = [
  new URL(
    "../src-tauri/gen/apple/build/sdb-native-m0_iOS.xcarchive",
    import.meta.url,
  ),
  new URL(
    "../src-tauri/gen/apple/build/arm64-sim/Smart%20Dartboard%20M0.app",
    import.meta.url,
  ),
];

for (const output of generatedOutputs) {
  await rm(fileURLToPath(output), { recursive: true, force: true });
}
