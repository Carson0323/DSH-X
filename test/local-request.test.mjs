import assert from 'node:assert/strict'
import test from 'node:test'

import { localRequestOk } from '../server.js'

const PORT = 3780
const local = `127.0.0.1:${PORT}`
const check = (headers) => localRequestOk({ headers }, PORT)

test('管理页自己发的请求放行', () => {
  assert.equal(check({ host: local }), true)
  assert.equal(check({ host: local, origin: `http://127.0.0.1:${PORT}` }), true)
  assert.equal(check({ host: `localhost:${PORT}`, origin: `http://localhost:${PORT}` }), true)
})

test('不带 Origin 的原生调用放行（DSH.exe 的托盘/唤醒）', () => {
  assert.equal(check({ host: local }), true)
  assert.equal(check({}), true)
})

test('别的网站发来的请求拒掉，浏览器对任何跨源 POST 都会带 Origin', () => {
  assert.equal(check({ host: local, origin: 'https://evil.example' }), false)
  assert.equal(check({ host: local, origin: 'http://evil.example' }), false)
  assert.equal(check({ host: local, origin: 'http://127.0.0.1.evil.example' }), false)
})

test('不透明来源（sandbox iframe、跨源跳转）拒掉', () => {
  assert.equal(check({ host: local, origin: 'null' }), false)
  assert.equal(check({ host: local, origin: 'file://' }), false)
})

test('别的回环端口也是别的源：dsh 页面不该能驱动启动器', () => {
  assert.equal(check({ host: local, origin: 'http://127.0.0.1:12810' }), false)
  assert.equal(check({ host: local, origin: 'http://localhost:1' }), false)
})

test('Host 不是本机就拒掉，挡 DNS rebinding', () => {
  assert.equal(check({ host: 'evil.example', origin: 'http://evil.example' }), false)
  assert.equal(check({ host: 'evil.example' }), false)
  assert.equal(check({ host: '127.0.0.1.evil.example:3780' }), false)
  assert.equal(check({ host: `127.0.0.1:${PORT + 1}` }), false)
})

test('https 的页面即使来自本机也不放行：管理器只说 http', () => {
  assert.equal(check({ host: local, origin: `https://127.0.0.1:${PORT}` }), false)
})
