@echo off
setlocal
cd /d "%~dp0"
for /f "usebackq tokens=*" %%i in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "OCR_VS=%%i"
if not defined OCR_VS exit /b 1
call "%OCR_VS%\VC\Auxiliary\Build\vcvars64.bat" >nul
if errorlevel 1 exit /b 1
if not exist "..\runtime\onnxruntime.dll" exit /b 1
if not exist "..\.deps\obj" mkdir "..\.deps\obj"
cl /nologo /std:c++20 /EHsc /utf-8 /O2 /MT /DNOMINMAX /DWIN32_LEAN_AND_MEAN /I "..\.deps\onnxruntime-win-x64-1.31.0\include" /Fo"..\.deps\obj\\" /Fe"..\runtime\echo-ocr.exe" main.cpp image.cpp recognizer.cpp capture.cpp /link /LIBPATH:"..\.deps\onnxruntime-win-x64-1.31.0\lib" onnxruntime.lib windowsapp.lib d3d11.lib dxgi.lib dwmapi.lib windowscodecs.lib bcrypt.lib ws2_32.lib user32.lib ole32.lib
exit /b %errorlevel%
