import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import vm from 'node:vm'
import { fileURLToPath } from 'node:url'

const html = readFileSync(fileURLToPath(new URL('../public/index.html', import.meta.url)), 'utf8')

/** 页面里那段没有打包器的内联脚本（改设置页时最容易碰坏它）。 */
const inlineScript = () => {
  const match = /<script>([\s\S]*?)<\/script>/.exec(html)
  assert.ok(match, '页面里应该有一段内联脚本')
  return match[1]
}

test('设置页有版本目录入口，放在高级设置里、复用现有字段样式', () => {
  assert.match(
    html,
    /<label class="field"><span>版本目录<\/span><input id="dataDir" type="text" \/><\/label>/,
    '输入框要与现有字段同款（.field + 同款 input 样式）',
  )
  assert.match(html, /<p class="hint" id="dataDirHint"><\/p>/, '提示行复用 .hint（空内容自动隐藏）')
  // 文本输入框本来就在样式表里，新控件不需要额外 CSS
  assert.match(html, /input\[type=text\], input\[type=number\], select \{/)
})

test('保存时把版本目录一起提交，留空表示不改', () => {
  assert.match(html, /const dirBefore = dataDirEl\.value\.trim\(\)/)
  assert.match(html, /\.\.\.\(dirBefore \? \{ dataDir: dirBefore \} : \{\}\)/)
})

test('读到的设置填进输入框，并说明插件/profile 位置与迁移语义', () => {
  assert.match(html, /if \('dataDir' in data\) \{[\s\S]*?dataDirEl\.value = String\(data\.dataDir \?\? ''\)/)
  assert.match(html, /dataDirHint\.textContent = `dsh 各版本装在这里/)
  assert.match(html, /插件和 profile 仍在/)
  assert.match(html, /已安装的版本不会自动迁移/)
})

test('改过目录的保存提示说明立即生效和不迁移', () => {
  // 末尾斜杠不该误判成"改过"：用户常带着 '\' 保存
  assert.match(html, /const normDir = \(value\) => String\(value \?\? ''\)\.trim\(\)\.replace\(\/\[\\\\\/\]\+\$\/, ''\)/)
  assert.match(html, /normDir\(data\.dataDir\) !== normDir\(dirBefore\)/)
  assert.match(html, /版本目录已改为 \$\{data\.dataDir\}（立即生效）/)
})

test('内联脚本仍能解析', () => {
  // 只编译不运行：语法坏了这里就炸，运行时的行为靠上面的结构断言看住
  new vm.Script(inlineScript())
})
