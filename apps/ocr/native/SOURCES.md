# OCR 资源来源

原生程序是本项目针对固定副词条行编写的识别与窗口捕获实现；没有捆绑 RapidOCR/RapidOcrOnnx 的旧二进制或 Python 包。

## PP-OCRv6 small recognition

- 来源：<https://huggingface.co/PaddlePaddle/PP-OCRv6_small_rec_onnx>
- 固定 revision：`b8f84f0b80c529de40b4fbb3544b84fa7233a513`
- 准备脚本直接从该 revision 下载 `inference.onnx` 和 `inference.yml`，不依赖 UmaOCR 或其他本地项目。
- 字典：`apps/ocr/assets/ppocrv6-small-dictionary.txt`，按此 revision 的 YAML `PostProcess.character_dict` 顺序导出，每个字符一行，UTF-8 无 BOM、LF 换行。Git 属性禁止换行转换，以保持固定摘要；更新模型时同步更新该字典，不能在旧模型上调整字符顺序。
- ONNX SHA-256：`5435fd747c9e0efe15a96d0b378d5bd157e9492ed8fd80edf08f30d02fa24634`
- YAML SHA-256：`ab078671bb49f06228eadccd34f1bb501e157f7a047095ffb943ba81512c77d1`
- 字典 SHA-256：`b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d`
- 授权：PaddleOCR Apache-2.0，附 `PaddleOCR-LICENSE.txt`。输入采用 BGR、CHW、48 像素高、等比例缩放／补零和 CTC 解码；字典前加 blank、后加 space，运行时验证输出类别数。

## ONNX Runtime

- 版本：1.31.0，官方 Windows x64 CPU SDK；不使用 GPU 包。
- 下载：<https://github.com/microsoft/onnxruntime/releases/tag/v1.31.0>
- SDK ZIP SHA-256：`3958d8a44984160692c1b4ffd0de3cb48f8c39f9d34bb31e433db06e30b82195`
- 仅随程序分发 `onnxruntime.dll`、官方 LICENSE 和 ThirdPartyNotices；头文件、导入库、调试符号留在被忽略的 `.deps/`。
- 将已安装 Visual Studio 的 Microsoft.VC143.CRT 中所需 DLL 随运行目录分发，避免依赖用户安装开发工具。

WIC 图片解码、Windows Graphics Capture、D3D11 和 SHA-256 使用 Windows 系统 API。程序没有 OpenCV 或 Python 运行依赖。
