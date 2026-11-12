// Kokteki mcp.test.js'i DEGISTIRMEDEN Rust binary'sine karsi kosturur: dosyayi okur,
// sadece spawn satirini NOTIX_RUST_BIN --mcp ile degistirip calistirir.
// Kullanim: NOTIX_RUST_BIN=<notix.exe> node mcp-js-wrapper.js
const fs = require('node:fs')
const path = require('node:path')

if (!process.env.NOTIX_RUST_BIN) throw new Error('NOTIX_RUST_BIN gerekli')
const root = path.join(__dirname, '..', '..', '..')
const from = "spawn(process.execPath, [path.join(__dirname, 'mcp.js')], {"
const src = fs.readFileSync(path.join(root, 'mcp.test.js'), 'utf8')
if (!src.includes(from)) throw new Error('mcp.test.js spawn satiri degismis, sarmalayici guncellenmeli')

new Function('require', '__dirname', src.replace(from, "spawn(process.env.NOTIX_RUST_BIN, ['--mcp'], {"))(require, root)
