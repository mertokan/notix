# Notix (Rust)

Electron surumunun yaninda duran, tamamen Rust ikinci surum. Ayni `notes/` klasorunu
ve ayni kurallari kullanir (bkz. `../AGENTS.md`); iki uygulama ayni klasoru acabilir.

- `core/` — `notix-core`: `store.js`'in karsiligi (satir tabanli yazma, `expect` → 409,
  silme yok / `.trash`, bellek bolumu, otomatik git yedegi).
- `app/` — `notix` binary'si. Simdilik sadece `notix --mcp`; arayuz sonra.

## Derleme ve test

```bash
cd rust
cargo build --release          # → target/release/notix(.exe)
cargo test --workspace         # node + git varsa JS ile esdegerlik testleri de kosar
cargo clippy --workspace -- -D warnings
```

Esdegerlik testleri `node`'u cagirir: ayni girdi dosyalarinda `store.js` ile Rust cikti
karsilastirilir, ve kokteki `mcp.test.js` degistirilmeden Rust binary'sine karsi kosar
(`app/tests/mcp-js-wrapper.js`).

## MCP ayari (Claude Desktop / Claude Code)

`NOTIX_DIR` not klasorudur; verilmezse `Belgeler/Notix`.

```json
{
  "mcpServers": {
    "notix": {
      "command": "C:\\yol\\notix.exe",
      "args": ["--mcp"],
      "env": { "NOTIX_DIR": "C:\\Users\\ben\\Documents\\Notix" }
    }
  }
}
```

Claude Code: `claude mcp add notix -e NOTIX_DIR=/yol/notes -- /yol/notix --mcp`

Yedek gecikmesi `NOTIX_BACKUP_MS` (varsayilan 20000).
