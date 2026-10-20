# 本机 OCR

Windows x64 桌面版安装了内置组件后，在“当前声骸”下选择“内置识别”，点击“开启 OCR 自动填入”。自动选择唯一的鸣潮窗口；多个窗口时，先停止识别、刷新窗口列表并选择目标。其他桌面平台隐藏内置识别选项，保留外部 UDP OCR；未安装内置组件的 Windows 版本默认使用外部 OCR。浏览器版不显示这些控件。

仅支持简体中文、16:9 游戏客户区：装备页识别右侧副词条，强化页识别左侧副词条，最多 5 条。两行主词条不识别，强化弹窗和动画页不作为目标页面。默认每 300ms 检查，页面、客户区尺寸及副词条连续两次一致才填入，不识别声骸名称来判断一致性；实际延迟取决于识别耗时及画面稳定程度。已确认结果不反复覆盖手动输入；切换页面、窗口恢复或重新开启后会重新确认。没有已解锁副词条时不会自动清空原有输入，请手动清空。

后台捕获绑定游戏窗口，失焦或被遮挡时可以继续取图，最小化或窗口不可用时暂停。窗口失效、画面比例不支持、捕获失败和进程退出会显示状态。所有识别均在本机 CPU 完成，不需要 Python、CUDA 或翻译模型。

已有 ok-ww 用户可选择“外部 ok-ww”，继续使用原集成任务向 `127.0.0.1:9999` 发送原有 `buffEntries` 格式。不要同时运行两个发送源。这里的内置识别只负责读取画面，不操作游戏。

## 准备与构建

需要 Windows x64、PowerShell 7、Rust 工具链、Visual Studio 2022 C++ x64 工具和 Windows 10/11 SDK。在仓库根目录运行以下命令即可准备并构建便携版，不依赖其他本地项目、Python 或 YAML 解析库：

```powershell
.\scripts\prepare-ocr.ps1
.\scripts\build-desktop.ps1
```

准备脚本从 PaddlePaddle 的固定 Hugging Face revision 下载识别模型和配置，从官方 GitHub Release 下载固定版本 ONNX Runtime CPU SDK；每个文件均校验 SHA-256。匹配的字典作为约 75 KB 的 UTF-8 文本资源保存在源码中。有效缓存可离线复用；损坏缓存会重新下载，失败下载不覆盖原文件。正常识别不会联网下载资源。

独立 OCR 运行目录为 `apps/ocr/runtime/`；便携桌面版生成在 `out/EchoStudio/`，包含桌面 EXE 和相邻 `ocr/` 文件夹。不要只复制桌面 EXE 而漏掉 `ocr/`。桌面 WebView 仍沿用 Tauri 的系统 WebView2 要求。

默认 `cargo build`、`cargo run --release` 和原有 macOS 打包流程保持不依赖 OCR 资源；没有安装相邻组件时使用外部 OCR。若要生成含内置 OCR 的 Windows NSIS 安装包，先运行准备脚本，再运行：

```powershell
.\scripts\build-ocr.ps1
cargo install tauri-cli --version '^2.0.0' --locked
Push-Location apps/desktop/src-tauri
try { cargo tauri build --config tauri.ocr.conf.json } finally { Pop-Location }
```

可选配置 `tauri.ocr.conf.json` 将完整运行目录映射为安装后的 `ocr/` 资源目录；Rust 通过应用资源路径定位它。只在 Windows x64 使用此配置，普通桌面构建不启用它。

本次 v6 small 识别模型为约 20.2 MiB，CPU DLL 为约 16.5 MiB；加上字典、配置、子进程、Visual C++ DLL 和许可证后，独立 OCR 目录约 **38.3 MiB**。构建脚本会打印实际体积；SDK 压缩包、开发头文件和编译中间文件不属于发布体积。

## 验证边界

固定截图验收使用本地 `captures/ocr/` 中的 6 张原图，截图为 1924×1126，客户区为 `(2,45,1920,1080)`。这些含用户信息的原图已忽略，不纳入 Git。以下测试中的 Python、Pillow、tkinter 仅为开发验收工具：

```powershell
python scripts/test-native-ocr.py
python scripts/test-ocr-capture.py
node scripts/test-ocr-ui.mjs
cargo test -p echo_calculator_app ocr::tests
```

图片回归覆盖 6 张原图、1600×900／1280×720 缩放及页面锚点缺失、空白和非 16:9 的拒绝行为。受控窗口验收覆盖真实 WGC 客户区裁剪、后台识别、UDP 结果、页面切换、最小化暂停、恢复重扫和退出。

上述固定图片和受控窗口结果不代表真实鸣潮游戏内验收；游戏内捕获权限、HDR、实际 UI 缩放及长时间运行仍须在游戏中确认。低置信度、陌生词条或非法数值会保留原输入并继续识别，不猜测修正。
