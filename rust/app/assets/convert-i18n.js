// public/i18n.js sozlugunu (S) assets/i18n.json'a cevirir. Calistir: node assets/convert-i18n.js
const fs = require('fs')
const path = require('path')
const src = fs.readFileSync(path.join(__dirname, '../../../public/i18n.js'), 'utf8')
const a = src.indexOf('const S = {')
const b = src.indexOf('  // Seciciler her dili')
const S = new Function('return ' + src.slice(a + 'const S = '.length, b).trim())()
fs.writeFileSync(path.join(__dirname, 'i18n.json'), JSON.stringify(S))
console.log(Object.keys(S).join(','), Object.keys(S.en).length, 'anahtar (en)')
for (const l of Object.keys(S)) {
  const eksik = Object.keys(S.tr).filter((k) => !(k in S[l]))
  if (eksik.length) console.log(l, 'eksik:', eksik.join(' '))
}
