# 上游合并工作流

## 为什么需要这套流程

LUODA-Codex 是从 [CodexPlusPlus](https://github.com/BigPizzaV3/CodexPlusPlus) fork 出来的品牌化版本。区别在于：

- **目录名**: codex-plus-* → luoda-codex-*
- **包名**: codex_plus_core → luoda_codex_core
- **UI 品牌**: 删除了 Zed远程项目/脚本市场/推荐内容 三个菜单项
- **品牌名**: Codex++ → L
- **URL**: BigPizzaV3 → luoda2023

不能直接 git merge upstream/main，因为路径和引用全对不上。

## 一键合并

`ash
cd LUODA-Codex
python scripts/merge-upstream.py              # 查看上游有多少新提交
python scripts/merge-upstream.py --apply       # 执行合并 + 品牌化 + 自动验证
python scripts/merge-upstream.py --verify-only # 只验证编译和测试
`

--apply 会自动执行：
1. 从上游拉取最新代码
2. 路径映射（codex-plus-* → luoda-codex-*）
3. 引用替换（包名、变量名等）
4. 品牌化补丁（删除已移除菜单、品牌化 UI 文字）
5. cargo check + cargo test 自动验证

## 品牌化映射表

| 原始（上游） | 替换（本地） |
|-------------|-------------|
| crates/codex-plus-core | crates/luoda-codex-core |
| crates/codex-plus-data | crates/luoda-codex-data |
| pps/codex-plus-launcher | pps/luoda-codex-launcher |
| pps/codex-plus-manager | pps/luoda-codex-manager |
| codex_plus_core | luoda_codex_core |
| codex_plus_data | luoda_codex_data |
| BigPizzaV3 | luoda2023 |

## 注意事项

- 文本文件（.rs/.tsx/.json 等）使用文本级替换，安全处理 UTF-8 中文
- 二进制文件（.ico/.png 等）使用字节级替换
- 合并前会自动备份被覆盖的文件到 .merge_backups/
- 合并后自动运行 cargo check 和 cargo test 验证

## 关键兼容性约束（不做品牌化替换）

以下内容因上游兼容性保留不变：
- ConfigOwnership::CodexPlusPlus — Rust 枚举 variant
- "CodexPlusPlus" — 遗留配置键名 LEGACY_RELAY_PROVIDERS
- codex_plus_bridge — 跨进程协议引用

## 新增功能时需要同步的文件

| 类型 | 文件 |
|------|------|
| Rust 后端 | crates/codex-plus-core/src/*.rs |
| Tauri 命令 | pps/codex-plus-manager/src-tauri/src/commands.rs |
| 前端组件 | pps/codex-plus-manager/src/components/*.tsx |
| 前端样式 | pps/codex-plus-manager/src/styles.css |
| 配置 | pps/codex-plus-manager/package.json |
| 构建 | Cargo.toml, .github/workflows/*.yml |

## 合并后手动检查清单

- [ ] cargo check 通过（自动）
- [ ] cargo test -p luoda-codex-core 通过（自动）
- [ ] 检查 git status 确认没有意外修改
- [ ] 运行 
pm run build（在 luoda-codex-manager 目录）
- [ ] 检查图标/Logo 是否正常显示
- [ ] UI 标题栏显示 "L" 而非 "Codex++"
- [ ] 检查"关于"页面品牌信息正确
