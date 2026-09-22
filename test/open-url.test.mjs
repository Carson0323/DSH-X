import assert from 'node:assert/strict'
import test from 'node:test'

import { canOpenLocally } from '../server.js'

const ok = [
  // 管理页自己：http://127.0.0.1:<port>/
  'http://127.0.0.1:3780/',
  'http://127.0.0.1:3780',
  'http://localhost:3780/',
  'http://127.0.0.1/',
  // dsh 的地址：http://127.0.0.1:<port>/?token=...
  'http://127.0.0.1:51234/?token=a1b2c3',
  'http://127.0.0.1:51234/?token=AbC-123_456',
  'http://localhost:51234/?token=AbC-123_456',
  'https://127.0.0.1:1/path?query=1#frag',
  'HTTP://127.0.0.1:3780/?window=1',
]

const bad = [
  // #4 的 PoC：前缀合法，& 之后是另一条命令
  'http://127.0.0.1:1/?&notepad',
  'http://127.0.0.1:1/?a=1&calc.exe',
  'http://127.0.0.1:1/?x=%PATH%',
  'http://127.0.0.1:1/?x=|calc',
  'http://127.0.0.1:1/?x=^&^<^>',
  'http://127.0.0.1:1/?x="&notepad',
  "http://127.0.0.1:1/?x='&notepad'",
  'http://127.0.0.1:1/?x=(1)',
  'http://127.0.0.1:1/?x=1 2',
  'http://127.0.0.1:1/?x=`whoami`',
  // 不是本机地址 / 不是 http(s)
  'http://evil.example/',
  'http://127.0.0.1.evil.example/',
  'file://127.0.0.1/C:/Windows/win.ini',
  'javascript:alert(1)',
  // userinfo：浏览器会把 @ 前的当用户信息、@ 后的当主机
  'http://user@127.0.0.1/',
  'http://127.0.0.1@evil.example/',
  // 控制字符
  'http://127.0.0.1:3780/?x=1\r\nSet-Cookie: a=b',
]

test('管理页和 dsh 的地址放行', () => {
  for (const url of ok) {
    assert.equal(canOpenLocally(url), true, url)
  }
})

test('#4 的 PoC 与 cmd 元字符一律拒绝', () => {
  for (const url of bad) {
    assert.equal(canOpenLocally(url), false, url)
  }
})

test('非字符串一律拒绝', () => {
  for (const value of [null, undefined, 42, {}, ['http://127.0.0.1:3780/']]) {
    assert.equal(canOpenLocally(value), false, JSON.stringify(value))
  }
})

test('放行的 URL 里不出现 cmd /c start 的特殊字符', () => {
  // 回归网：白名单一旦被改宽，先把 cmd 的语句分隔符和变量展开挡在外面
  const meta = /[&|^<>()%"'\r\n\t`]/
  for (const url of ok) {
    assert.equal(meta.test(url), false, url)
  }
})
