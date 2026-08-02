import { readFile } from 'node:fs/promises';

const read=path=>readFile(new URL(`../${path}`,import.meta.url),'utf8');
const matrix=JSON.parse(await read('docs/PLATFORM_MATRIX.json'));
const [rustToolchain,workspace,nativeCargo,nativePackage,nativeLock,rootLock,
  storageCargo,tauriConfig,xcodeProject,macLifecycle,rustDocker,bleDocker,ci,release]=await Promise.all([
  read('rust-toolchain.toml'),read('Cargo.toml'),read('apps/tauri/src-tauri/Cargo.toml'),
  read('apps/tauri/package.json'),read('apps/tauri/package-lock.json'),read('Cargo.lock'),
  read('crates/storage/Cargo.toml'),read('apps/tauri/src-tauri/tauri.conf.json'),
  read('apps/tauri/src-tauri/gen/apple/sdb-native-m0.xcodeproj/project.pbxproj'),
  read('apps/tauri/src-tauri/gen/apple/Sources/sdb-native-m0/AppLifecycleHost.mm'),
  read('Dockerfile.rust'),read('Dockerfile.ble'),read('.github/workflows/ci.yml'),
  read('.github/workflows/container-release.yml'),
]);

const failures=[];
const expect=(condition,message)=>{ if(!condition) failures.push(message); };
const quoted=(text,key)=>text.match(new RegExp(`^${key}\\s*=\\s*"([^"]+)"`,'m'))?.[1];
const inlineVersion=(text,key)=>text
  .match(new RegExp(`^${key}\\s*=\\s*\\{[^}]*version\\s*=\\s*"([^"]+)"`,'m'))?.[1];
const packageVersion=(lock,name)=>lock
  .match(new RegExp(`name = "${name}"\\nversion = "([^"]+)"`))?.[1];

expect(matrix.schema_version===1,'matrix schema_version must be 1');
expect(process.versions.node.split('.')[0]===matrix.toolchains.node.major,'executing Node major drift');
expect(quoted(rustToolchain,'channel')===matrix.toolchains.rust.toolchain,'Rust toolchain drift');
expect(quoted(workspace,'edition')===matrix.toolchains.rust.edition,'Rust edition drift');
expect(quoted(workspace,'rust-version')===matrix.toolchains.rust.msrv,'workspace MSRV drift');
expect(quoted(nativeCargo,'rust-version')===matrix.toolchains.rust.msrv,'native MSRV drift');

const packageJson=JSON.parse(nativePackage);
const packageLock=JSON.parse(nativeLock);
const tauriJson=JSON.parse(tauriConfig);
expect(packageJson.engines.node===`${matrix.toolchains.node.major}.x`,'Node engine drift');
expect(packageJson.engines.npm===`${matrix.toolchains.node.npm_major}.x`,'npm engine drift');
expect(packageLock.packages[''].engines.node===packageJson.engines.node,'npm lock Node engine drift');
expect(packageJson.devDependencies['@tauri-apps/cli']===matrix.toolchains.tauri.cli,'Tauri CLI drift');
expect(packageJson.devDependencies['@playwright/test']===matrix.toolchains.playwright,'Playwright drift');
expect(packageJson.scripts['build:macos:app']==='tauri build --bundles app --no-sign --ci',
  'macOS app bundle command drift');
expect(inlineVersion(nativeCargo,'tauri')===matrix.toolchains.tauri.rust_runtime,
  'Tauri Rust runtime drift');
expect(packageVersion(rootLock,'rusqlite')===matrix.toolchains.sqlite.rusqlite,'rusqlite drift');
expect(packageVersion(rootLock,'libsqlite3-sys')===matrix.toolchains.sqlite.libsqlite3_sys,
  'libsqlite3-sys drift');
expect(storageCargo.includes('features = ["backup", "bundled"]'),'SQLite feature drift');

const rustImage=matrix.toolchains.rust.toolchain.split('.').slice(0,2).join('.');
expect(rustDocker.startsWith(`FROM rust:${rustImage}-bookworm`),'Rust Docker builder drift');
expect(rustDocker.includes('FROM debian:bookworm-slim'),'Rust Docker distribution drift');
expect(bleDocker.startsWith(`FROM python:${matrix.toolchains.python_legacy.major_minor}-slim`),
  'BLE gateway Python drift');
expect(ci.includes(`node-version: "${matrix.toolchains.node.major}"`),'CI Node drift');
expect(ci.includes(`toolchain: ${matrix.toolchains.rust.toolchain}`),'CI Rust drift');
expect(ci.includes(`grep -F 'Xcode ${matrix.toolchains.apple_build_baseline.xcode}'`),'CI Xcode drift');
expect(ci.includes(`xcode-version: "${matrix.toolchains.apple_build_baseline.xcode}"`),
  'CI Xcode selection drift');
expect(ci.includes('playwright install chromium webkit'),'CI browser installation drift');
expect(ci.includes('run test:chromium') && ci.includes('run test:webkit'),'CI browser coverage drift');
expect(ci.includes('run build:macos:app') && ci.includes('Smart-Dartboard-macOS-unsigned.zip'),
  'CI macOS app bundle drift');
expect(packageJson.scripts['test:ios:lifecycle']==='node scripts/test-ios-lifecycle.mjs'
  && ci.includes('run test:ios:lifecycle'),'iOS lifecycle test drift');
expect(macLifecycle.includes('NSWorkspaceWillSleepNotification')
  && macLifecycle.includes('NSWorkspaceDidWakeNotification')
  && macLifecycle.includes('sdb_app_sleep_changed'),
  'macOS sleep/wake lifecycle adapter drift');
expect(release.includes('platforms: linux/amd64,linux/arm64'),'container architecture drift');
expect(tauriJson.bundle.macOS.minimumSystemVersion===matrix.platforms.macos.minimum,
  'macOS deployment target drift');
expect(tauriJson.identifier==='de.gammaproduction.smart-dartboard',
  'product bundle identifier drift');
const iosTargets=[...xcodeProject.matchAll(/IPHONEOS_DEPLOYMENT_TARGET = ([^;]+);/g)]
  .map(match=>match[1]);
expect(iosTargets.length>0 && iosTargets.every(value=>value===matrix.platforms.ios_ipados.minimum),
  'iOS deployment target drift');

for(const [platform,entry] of Object.entries(matrix.platforms)){
  expect(entry.build && entry.installation && entry.hardware,`${platform} lacks evidence states`);
  if(entry.support_level==='supported'){
    expect(entry.hardware==='qualified',`${platform} claims support without hardware qualification`);
  }
}
expect(Array.isArray(matrix.qualified_hardware),'qualified_hardware must be an array');

if(failures.length){
  console.error(failures.map(failure=>`- ${failure}`).join('\n'));
  process.exit(1);
}
console.log('Platform matrix matches pinned toolchains, targets, containers and workflows.');
