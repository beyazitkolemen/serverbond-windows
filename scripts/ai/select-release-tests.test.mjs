import assert from 'node:assert/strict'
import test from 'node:test'
import { changedPackageIds, selectReleaseTests } from './select-release-tests.mjs'

const core = 'crates/serverbond-core/'

test('database inventory change runs only its real MySQL credential test', () => {
  assert.deepEqual(selectReleaseTests([`${core}src/database_inventory.rs`]), ['mysql_credentials'])
})

test('a MySQL package change covers all three affected live runtimes', () => {
  assert.deepEqual(selectReleaseTests([`${core}catalog.json`], new Set(['mysql'])), [
    'mysql_credentials', 'environment', 'preferences_runtime',
  ])
})

test('package version changes are detected even when the id line is unchanged', () => {
  const previous = JSON.stringify([{ id: 'mysql', version: '8.4.9' }, { id: 'php', version: '8.4' }])
  const current = JSON.stringify([{ id: 'mysql', version: '8.4.10' }, { id: 'php', version: '8.4' }])
  assert.deepEqual([...changedPackageIds(previous, current)], ['mysql'])
})

test('unrelated release and UI changes do not start network service tests', () => {
  assert.deepEqual(selectReleaseTests(['docs/releases/v1.3.12.md', 'src/App.tsx']), [])
})

test('PostgreSQL and Cloudflared changes select their own tests', () => {
  assert.deepEqual(selectReleaseTests([`${core}src/postgres.rs`], new Set(['cloudflared'])), [
    'cloudflared_launch', 'postgres',
  ])
})
