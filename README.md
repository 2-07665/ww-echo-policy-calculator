# 鸣潮声骸强化策略计算器

基于动态规划与 λ-search 的策略求解器，用于计算《鸣潮》中声骸强化的最优决策。核心算法与数据在 `policy_core/`，示例见 `echo_calculator.ipynb`。提供两种便捷前端：
- PyWebview 桌面端（`webview_UI/`）：轻量、单窗口桌面应用。
- Streamlit 网页端（`streamlit_UI/`）：浏览器访问，支持局域网分享。

## 环境要求
- Python 3.10+
- 跨平台支持：
  - 求解器与 Streamlit / PyWebview 前端：Windows、macOS、Linux
  - OCR 模块：仅 Windows（依赖 Win32 API）

## 安装
求解器本身无需依赖即可使用。

可按需安装前端依赖：
```bash
python3 -m venv .venv
source .venv/bin/activate

pip install pywebview
#或
pip install streamlit
```

可选：安装 OCR 依赖（仅支持Windows平台与PyWebview）
```bash
pip install -r requirements_ocr.txt
```

## 快速开始
### PyWebview
```bash
python webview_UI/app.py
```
启动支持OCR的前端
```bash
python webview_UI/app_ocr.py
```

### Streamlit
```bash
streamlit run streamlit_UI/app.py
```

### 统计数据记录器（可选）
Streamlit UI 提供了一个用于“点击计数”的小工具页面，方便自行补充各词条出现频次：
```bash
streamlit run streamlit_UI/app_count.py
```
数据保存到 `streamlit_UI/user_counts_data.json`，主页面中勾选“在计算中包含自定义统计数据”即可叠加计算。

## 打包桌面应用（PyWebview）
安装 PyInstaller：
```bash
pip install pyinstaller
```

执行构建脚本：
```bash
python scripts/build_webview.py
```

输出位于 `dist/` 目录。

## 致谢（Acknowledgements）

### 赞助
特别感谢 Bilibili **[@冬葳蕤](https://space.bilibili.com/58999432)** 自发组织的库洛游戏二创激励计划为本项目提供的赞助。

### 词条统计数据来源
本项目使用的副词条产出统计数据来自：
- Bilibili **[@IceHe何瀚清](https://space.bilibili.com/13378662)**

### 讨论与交流
- Bilibili **[@冰封_](https://space.bilibili.com/88548986)**
