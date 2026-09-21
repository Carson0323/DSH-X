import assert from 'node:assert/strict'
import test from 'node:test'

import { versionsToKeep } from '../server.js'

const sorted = (set) => [...set].sort()

test('装旧版不会把最新版清掉', () => {
  // 已经装了 0.1.13 和 0.1.12，用户在下拉里挑了 0.1.11，
  // install() 会把刚装的那个插到 config.versions 最前面
  assert.deepEqual(sorted(versionsToKeep(['0.1.11', '0.1.12', '0.1.13'])), ['0.1.11', '0.1.13'])
})

test('装新版保留刚装的 + 上一个（回退用）', () => {
  assert.deepEqual(sorted(versionsToKeep(['0.1.13', '0.1.12', '0.1.11'])), ['0.1.12', '0.1.13'])
})

test('正在跑的那个一定留下', () => {
  assert.deepEqual(
    sorted(versionsToKeep(['0.1.13', '0.1.12', '0.1.11'], '0.1.11')),
    ['0.1.11', '0.1.12', '0.1.13'],
  )
})

test('预发布版按 semver 比，不按字符串比', () => {
  // alpha.10 比 alpha.2 新，所以它该排在 alpha.2 前面、被优先保留
  assert.deepEqual(
    sorted(versionsToKeep(['0.1.6-alpha.2', '0.1.6-alpha.10', '0.1.5'])),
    ['0.1.6-alpha.10', '0.1.6-alpha.2'],
  )
})

test('认不出版本的项不会被当成最新', () => {
  assert.deepEqual(sorted(versionsToKeep(['0.1.13', 'bogus', '0.1.12'])), ['0.1.12', '0.1.13'])
})

test('空列表不炸', () => {
  assert.equal(versionsToKeep([]).size, 0)
})

test('limit 决定保留几个', () => {
  assert.equal(versionsToKeep(['0.1.13', '0.1.12', '0.1.11'], null, 1).size, 1)
})
