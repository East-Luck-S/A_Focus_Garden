# 参与贡献

Focus Garden 是 Windows 10/11 桌面应用。请保持 Rust、React 与 SQLite 的正式职责边界，不要把业务真值移动到前端。

## 开发原则

- Rust 负责 Session 状态机、计时真值、植物生成、事务和持久化。
- React 只负责呈现、交互、投影、SVG 渲染和瞬态 UI 状态。
- SQLite 是唯一的业务持久化真值来源。
- Schema 变更只能追加 migration；不得改写历史 migration。
- 不得通过删除或重置数据库处理错误或测试失败。
- 不得把真实用户数据库、签名证书、令牌、密钥或构建机路径提交到仓库。

## 本地验证

```powershell
npm ci
npm run licenses:verify
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo check --manifest-path src-tauri/Cargo.toml --locked
cargo test --manifest-path src-tauri/Cargo.toml --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --locked -- -D warnings
```

涉及原生运行、数据库或 Tauri command 的改动还必须在 Windows 上验证真实应用。涉及发布产物时运行：

```powershell
npm run release:build
```

只有带有效 Authenticode 签名、来自干净源码树且通过人工发布矩阵的产物，才可以运行 `npm run release:verify:public` 并作为公开发行候选。

## 提交与 Pull Request

- 每个提交只表达一个清晰的逻辑变化。
- PR 说明应包含动机、数据/迁移影响、测试证据和可见 UI 变化。
- 如果改动依赖用户数据迁移，必须提供非破坏性的升级与回滚失败策略。
- 依赖变更必须重新生成并审查 `THIRD_PARTY_NOTICES.txt`。
