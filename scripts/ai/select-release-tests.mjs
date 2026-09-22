// Choose only live service tests affected since the preceding release.
import { execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const core = 'crates/serverbond-core/'
const catalog = `${core}catalog.json`
const tools = `${core}tools.json`
const order = [
  'mysql_credentials',
  'environment',
  'preferences_runtime',
  'php_matrix',
  'branding_runtime',
  'cloudflared_launch',
  'postgres',
]

export function selectReleaseTests(changedFiles, changedPackages = new Set()) {
  const changed = new Set(changedFiles)
  const selected = new Set()
  const add = (...names) => names.forEach((name) => selected.add(name))
  const has = (...names) => names.some((name) => changed.has(`${core}${name}`))

  for (const name of order) {
    if (name !== 'postgres' && has(`tests/${name}.rs`)) add(name)
  }
  if (has('src/database_inventory.rs', 'src/cloud/database_inventory.rs')) {
    add('mysql_credentials')
  }
  if (has('src/services.rs', 'src/cloud/mysql.rs')) {
    add('mysql_credentials', 'environment')
  }
  if (has('src/preferences.rs')) add('preferences_runtime')
  if (has('src/postgres.rs', 'src/cloud/postgres.rs')) add('postgres')
  if (has('src/install.rs')) add(...order)

  if (changedPackages.has('mysql')) {
    add('mysql_credentials', 'environment', 'preferences_runtime')
  }
  if (changedPackages.has('postgres')) add('postgres')
  if (changedPackages.has('php')) {
    add('environment', 'preferences_runtime', 'php_matrix', 'branding_runtime')
  }
  if (changedPackages.has('caddy')) {
    add('environment', 'preferences_runtime', 'php_matrix', 'branding_runtime')
  }
  if (changedPackages.has('composer')) {
    add('environment', 'php_matrix', 'branding_runtime')
  }
  if (changedPackages.has('phpmyadmin')) add('environment')
  if (changedPackages.has('cloudflared')) add('cloudflared_launch')

  return order.filter((name) => selected.has(name))
}

function git(...args) {
  return execFileSync('git', args, { encoding: 'utf8' })
}

export function changedPackageIds(before, after) {
  const previous = new Map(JSON.parse(before).map((item) => [item.id, item]))
  const current = new Map(JSON.parse(after).map((item) => [item.id, item]))
  return new Set([...previous.keys(), ...current.keys()].filter(
    (id) => JSON.stringify(previous.get(id)) !== JSON.stringify(current.get(id)),
  ))
}

export function selectBetweenRefs(previous, current) {
  const files = git('diff', '--name-only', previous, current).trim().split('\n').filter(Boolean)
  const packages = new Set()
  for (const file of [catalog, tools]) {
    if (!files.includes(file)) continue
    for (const id of changedPackageIds(git('show', `${previous}:${file}`), git('show', `${current}:${file}`))) {
      packages.add(id)
    }
  }
  return selectReleaseTests(files, packages)
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 4) {
    console.error('Usage: node select-release-tests.mjs PREVIOUS_TAG CURRENT_REF')
    process.exit(2)
  }
  for (const name of selectBetweenRefs(process.argv[2], process.argv[3])) {
    console.log(name)
  }
}
