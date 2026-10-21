#include "ocr.hpp"
#include <wincodec.h>
#include <winrt/base.h>
#include <bcrypt.h>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>
#include <stdexcept>

std::string utf8(const std::wstring& text) {
    if (text.empty()) return {};
    int n = WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, text.data(), (int)text.size(), nullptr, 0, nullptr, nullptr);
    if (!n) throw std::runtime_error("Invalid Unicode");
    std::string out(n, '\0');
    WideCharToMultiByte(CP_UTF8, 0, text.data(), (int)text.size(), out.data(), n, nullptr, nullptr);
    return out;
}
std::string quote(const std::string& text) {
    std::string out = "\"";
    for (unsigned char c : text) {
        if (c == '\"' || c == '\\') { out += '\\'; out += c; }
        else if (c >= 32) out += c;
    }
    return out + "\"";
}
void status(const char* code, const std::string& detail) {
    std::cout << "{\"status\":" << quote(code) << ",\"detail\":" << quote(detail) << "}" << std::endl;
}
Image Image::crop(Rect r) const {
    if (r.x < 0 || r.y < 0 || r.w < 1 || r.h < 1 || r.x + r.w > w || r.y + r.h > h)
        throw std::runtime_error("Image crop outside client bounds");
    Image out{r.w, r.h, std::vector<unsigned char>((size_t)r.w*r.h*4)};
    for (int y = 0; y < r.h; ++y)
        memcpy(out.pixels.data() + (size_t)y*r.w*4, pixels.data() + ((size_t)(r.y+y)*w+r.x)*4, (size_t)r.w*4);
    return out;
}
Image Image::load(const std::filesystem::path& path) {
    winrt::com_ptr<IWICImagingFactory> factory;
    winrt::check_hresult(CoCreateInstance(CLSID_WICImagingFactory, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(factory.put())));
    winrt::com_ptr<IWICBitmapDecoder> decoder;
    winrt::check_hresult(factory->CreateDecoderFromFilename(path.c_str(), nullptr, GENERIC_READ, WICDecodeMetadataCacheOnDemand, decoder.put()));
    winrt::com_ptr<IWICBitmapFrameDecode> frame;
    winrt::check_hresult(decoder->GetFrame(0, frame.put()));
    UINT w, h;
    winrt::check_hresult(frame->GetSize(&w, &h));
    if (!w || !h || w > 8192 || h > 8192) throw std::runtime_error("Unsupported image size");
    winrt::com_ptr<IWICFormatConverter> converter;
    winrt::check_hresult(factory->CreateFormatConverter(converter.put()));
    winrt::check_hresult(converter->Initialize(frame.get(), GUID_WICPixelFormat32bppBGRA, WICBitmapDitherTypeNone, nullptr, 0, WICBitmapPaletteTypeCustom));
    Image out{(int)w, (int)h, std::vector<unsigned char>((size_t)w*h*4)};
    winrt::check_hresult(converter->CopyPixels(nullptr, w*4, (UINT)out.pixels.size(), out.pixels.data()));
    return out;
}
void Image::save(const std::filesystem::path& path) const {
    winrt::com_ptr<IWICImagingFactory> factory;
    winrt::check_hresult(CoCreateInstance(CLSID_WICImagingFactory, nullptr, CLSCTX_INPROC_SERVER, IID_PPV_ARGS(factory.put())));
    winrt::com_ptr<IWICStream> stream;
    winrt::check_hresult(factory->CreateStream(stream.put()));
    winrt::check_hresult(stream->InitializeFromFilename(path.c_str(), GENERIC_WRITE));
    winrt::com_ptr<IWICBitmapEncoder> encoder;
    winrt::check_hresult(factory->CreateEncoder(GUID_ContainerFormatPng, nullptr, encoder.put()));
    winrt::check_hresult(encoder->Initialize(stream.get(), WICBitmapEncoderNoCache));
    winrt::com_ptr<IWICBitmapFrameEncode> frame;
    winrt::check_hresult(encoder->CreateNewFrame(frame.put(), nullptr));
    winrt::check_hresult(frame->Initialize(nullptr));
    winrt::check_hresult(frame->SetSize(w, h));
    WICPixelFormatGUID format = GUID_WICPixelFormat32bppBGRA;
    winrt::check_hresult(frame->SetPixelFormat(&format));
    winrt::check_hresult(frame->WritePixels(h, w*4, (UINT)pixels.size(), const_cast<BYTE*>(pixels.data())));
    winrt::check_hresult(frame->Commit());
    winrt::check_hresult(encoder->Commit());
}
std::string sha256(const std::filesystem::path& path) {
    std::ifstream file(path, std::ios::binary);
    if (!file) throw std::runtime_error("Missing OCR resource: " + utf8(path.wstring()));
    BCRYPT_ALG_HANDLE algorithm = nullptr;
    BCRYPT_HASH_HANDLE hash = nullptr;
    if (BCryptOpenAlgorithmProvider(&algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0) throw std::runtime_error("SHA256 unavailable");
    auto cleanup = [&] { if (hash) BCryptDestroyHash(hash); BCryptCloseAlgorithmProvider(algorithm, 0); };
    if (BCryptCreateHash(algorithm, &hash, nullptr, 0, nullptr, 0, 0) < 0) { cleanup(); throw std::runtime_error("SHA256 initialization failed"); }
    char buffer[65536];
    while (file.read(buffer, sizeof buffer) || file.gcount()) {
        if (BCryptHashData(hash, reinterpret_cast<PUCHAR>(buffer), (ULONG)file.gcount(), 0) < 0) { cleanup(); throw std::runtime_error("SHA256 failed"); }
    }
    unsigned char digest[32];
    auto result = BCryptFinishHash(hash, digest, sizeof digest, 0);
    cleanup();
    if (result < 0) throw std::runtime_error("SHA256 failed");
    std::ostringstream out;
    for (unsigned char b : digest) out << std::hex << std::setfill('0') << std::setw(2) << (int)b;
    return out.str();
}
