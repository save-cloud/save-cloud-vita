![save cloud](./screenshot.png)

# Save Cloud for PSVita

> 游戏存档云备份工具

## 功能

- 游戏存档备份（本地/云盘)
- 文件管理（本地/云盘)
- ...等

## VitaShell 扫码安装

![qrcode](./qrcode.png)

## 项目结构

- PSVita SDK
  - [vitasdk](https://github.com/vitasdk)
- rust
  - [vita-rust](https://github.com/vita-rust)
- UI
  - > 没有使用 UI 框架，界面使用 [`libvita2d`](https://github.com/xerpi/libvita2d) 绘制
  - > TODO: 使用 [dioxus](https://github.com/DioxusLabs/dioxus) 重构
- 网络
  - ureq

## Build

- `cargo vita build vpk --release`
  - > 如果需要构建 vpk，需要实现 [ `save_cloud_api`](./src/api.rs)
  - > 或者删除 `save_cloud_api` 相关部分

## Reference

- [vitasdk](https://github.com/vitasdk)
- [vita-rust](https://github.com/vita-rust)
- [vitashell](https://github.com/TheOfficialFloW/VitaShell)
- [psdevwiki](https://www.psdevwiki.com/vita/)
