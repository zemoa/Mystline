import { readFileSync } from 'node:fs'

const tag = process.argv[2]
if (!/^\d+\.\d+\.\d+$/.test(tag ?? '')) throw new Error('Expected a release tag X.Y.Z')
const jsonFiles = ['package.json', 'frontend/package.json', 'src-tauri/tauri.conf.json']
const versions = jsonFiles.map(path => [path, JSON.parse(readFileSync(path, 'utf8')).version])
for (const path of ['Cargo.toml', 'src-tauri/Cargo.toml']) {
  const packageSection = readFileSync(path, 'utf8').split('[package]')[1]?.split('\n[')[0]
  versions.push([path, packageSection?.match(/^version\s*=\s*"([^"]+)"/m)?.[1]])
}
for (const [path, version] of versions) {
  if (version !== tag) throw new Error(`${path}: version ${version} does not match tag ${tag}`)
}
console.log(`Release ${tag}: all manifest versions match`)
