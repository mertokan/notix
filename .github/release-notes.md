Notix comes in two flavors. Both read and write the same plain-markdown notes folder (`Documents/Notix` by default), so you can switch between them at any time.

| | Electron | Rust |
|---|---|---|
| Download | `Notix-Setup-*.exe` (Windows), `.dmg` (macOS), `.AppImage` (Linux) | `notix-rs-*-windows-x64.zip`, `-macos-arm64.tar.gz`, `-linux-x64.tar.gz` |
| Install | installer | portable: unzip and run `notix` |
| Size / memory | larger | smaller, native UI |
| MCP server for AI agents | Settings → AI agents → Copy MCP config | `notix --mcp` (same in Settings) |

Builds are not code-signed yet: Windows SmartScreen may warn ("More info → Run anyway"), and on macOS use right-click → Open.

---

Notix iki sürümde geliyor. İkisi de aynı markdown not klasörünü (varsayılan `Belgeler/Notix`) kullanır; istediğin zaman birinden diğerine geçebilirsin. **Electron**: kurulum dosyası. **Rust**: kurulumsuz tek dosya, daha küçük ve yerel arayüz. Derlemeler henüz imzalı değil.
