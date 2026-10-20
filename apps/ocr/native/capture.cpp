#include "ocr.hpp"
#include <d3d11.h>
#include <dxgi1_2.h>
#include <dwmapi.h>
#include <windows.graphics.capture.interop.h>
#include <windows.graphics.directx.direct3d11.interop.h>
#include <winrt/Windows.Foundation.h>
#include <winrt/Windows.Graphics.Capture.h>
#include <winrt/Windows.Graphics.DirectX.h>
#include <winrt/Windows.Graphics.DirectX.Direct3D11.h>
#include <condition_variable>
#include <mutex>
#include <cwctype>
#include <stdexcept>

using namespace winrt;
using winrt::Windows::Graphics::SizeInt32;
using winrt::Windows::Graphics::Capture::GraphicsCaptureItem;
using winrt::Windows::Graphics::Capture::GraphicsCaptureSession;
using winrt::Windows::Graphics::Capture::Direct3D11CaptureFramePool;
using winrt::Windows::Graphics::Capture::Direct3D11CaptureFrame;
using winrt::Windows::Graphics::DirectX::DirectXPixelFormat;
using winrt::Windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;

bool is_game_title(std::wstring title) {
    auto first=title.find_first_not_of(L" \t\r\n\u00a0\u3000");
    if (first==std::wstring::npos) return false;
    auto last=title.find_last_not_of(L" \t\r\n\u00a0\u3000");
    title=title.substr(first,last-first+1);
    return title==L"鸣潮" || title==L"Wuthering Waves";
}
std::vector<Window> game_windows() {
    std::vector<Window> windows;
    EnumWindows([](HWND hwnd, LPARAM data)->BOOL {
        if (!IsWindowVisible(hwnd) || GetWindow(hwnd,GW_OWNER)) return TRUE;
        wchar_t title[512]{};
        GetWindowTextW(hwnd,title,512);
        if (!is_game_title(title)) return TRUE;
        DWORD pid=0;
        GetWindowThreadProcessId(hwnd,&pid);
        HANDLE process=OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION,FALSE,pid);
        if (!process) return TRUE;
        wchar_t path[32768]; DWORD length=32768;
        bool matched=false;
        if (QueryFullProcessImageNameW(process,0,path,&length)) {
            auto name=std::filesystem::path(std::wstring(path,length)).filename().wstring();
            matched=name==L"Client-Win64-Shipping.exe" || name==L"Wuthering Waves.exe";
        }
        CloseHandle(process);
        if (matched) reinterpret_cast<std::vector<Window>*>(data)->push_back({hwnd,title});
        return TRUE;
    },reinterpret_cast<LPARAM>(&windows));
    return windows;
}
struct Capture::Impl : std::enable_shared_from_this<Capture::Impl> {
    HWND hwnd;
    com_ptr<ID3D11Device> device;
    com_ptr<ID3D11DeviceContext> context;
    IDirect3DDevice graphics{nullptr};
    GraphicsCaptureItem item{nullptr};
    Direct3D11CaptureFramePool pool{nullptr};
    GraphicsCaptureSession session{nullptr};
    event_token frameToken{}, closeToken{};
    std::mutex mutex;
    std::condition_variable changed;
    Image latest;
    Direct3D11CaptureFrame pending{nullptr};
    bool closed=false;
    bool stopped=false;
    std::string error;
    SizeInt32 size{};
    explicit Impl(HWND window) : hwnd(window) {
        if (!GraphicsCaptureSession::IsSupported()) throw std::runtime_error("Windows Graphics Capture unavailable");
        D3D_FEATURE_LEVEL level;
        check_hresult(D3D11CreateDevice(nullptr,D3D_DRIVER_TYPE_HARDWARE,nullptr,D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                                       nullptr,0,D3D11_SDK_VERSION,device.put(),&level,context.put()));
        auto dxgi=device.as<IDXGIDevice>();
        com_ptr<IInspectable> inspectable;
        check_hresult(CreateDirect3D11DeviceFromDXGIDevice(dxgi.get(),inspectable.put()));
        graphics=inspectable.as<IDirect3DDevice>();
        auto interop=get_activation_factory<GraphicsCaptureItem,IGraphicsCaptureItemInterop>();
        check_hresult(interop->CreateForWindow(hwnd,guid_of<GraphicsCaptureItem>(),put_abi(item)));
        size=item.Size();
        pool=Direct3D11CaptureFramePool::CreateFreeThreaded(graphics,DirectXPixelFormat::B8G8R8A8UIntNormalized,2,size);
    }
    void start() {
        auto weak=weak_from_this();
        closeToken=item.Closed([weak](auto const&,auto const&) {
            if (auto state=weak.lock()) { std::lock_guard guard(state->mutex); state->closed=true; state->changed.notify_all(); }
        });
        frameToken=pool.FrameArrived([weak](auto const& sender,auto const&) {
            if (auto state=weak.lock()) state->receive(sender);
        });
        session=pool.CreateCaptureSession(item);
        session.IsCursorCaptureEnabled(false);
        session.StartCapture();
    }
    void receive(Direct3D11CaptureFramePool const& sender) {
            std::lock_guard guard(mutex);
            if (closed) return;
            try {
                auto frame=sender.TryGetNextFrame();
                if (!frame) return;
                auto content=frame.ContentSize();
                if (content.Width!=size.Width || content.Height!=size.Height) {
                    latest={}; pending=nullptr; size=content; frame.Close();
                    if (size.Width>0 && size.Height>0) sender.Recreate(graphics,DirectXPixelFormat::B8G8R8A8UIntNormalized,2,size);
                    return;
                }
                // Keep the newest GPU frame; CPU readback happens only when scanning.
                // Unlike throttling callbacks, this never drops the last static update.
                pending=std::move(frame); error.clear();
            } catch (const hresult_error& e) { error=to_string(e.message()); }
              catch (const std::exception& e) { error=e.what(); }
            changed.notify_all();
    }
    void read_pixels() {
        if (!pending) return;
        auto content=pending.ContentSize();
        auto access=pending.Surface().as<::Windows::Graphics::DirectX::Direct3D11::IDirect3DDxgiInterfaceAccess>();
        com_ptr<ID3D11Texture2D> texture;
        check_hresult(access->GetInterface(__uuidof(ID3D11Texture2D),texture.put_void()));
        D3D11_TEXTURE2D_DESC desc{}; texture->GetDesc(&desc);
        if (content.Width<=0 || content.Height<=0 || content.Width>(int)desc.Width || content.Height>(int)desc.Height)
            throw std::runtime_error("Capture texture size mismatch");
        desc.Usage=D3D11_USAGE_STAGING; desc.BindFlags=0; desc.CPUAccessFlags=D3D11_CPU_ACCESS_READ; desc.MiscFlags=0;
        com_ptr<ID3D11Texture2D> staging;
        check_hresult(device->CreateTexture2D(&desc,nullptr,staging.put()));
        context->CopyResource(staging.get(),texture.get());
        Image image{content.Width,content.Height,std::vector<unsigned char>((size_t)content.Width*content.Height*4)};
        D3D11_MAPPED_SUBRESOURCE mapped{};
        check_hresult(context->Map(staging.get(),0,D3D11_MAP_READ,0,&mapped));
        for (int y=0;y<content.Height;++y)
            memcpy(image.pixels.data()+(size_t)y*content.Width*4,(unsigned char*)mapped.pData+(size_t)y*mapped.RowPitch,(size_t)content.Width*4);
        context->Unmap(staging.get(),0);
        latest=std::move(image); pending.Close(); pending=nullptr;
    }
    void stop() {
        { std::lock_guard guard(mutex); if (stopped) return; stopped=true; closed=true; pending=nullptr; }
        // Unregister callbacks before releasing any state they can access.
        if (item) item.Closed(closeToken);
        if (pool) pool.FrameArrived(frameToken);
        if (session) session.Close();
        if (pool) pool.Close();
    }
    ~Impl() { try { stop(); } catch (...) {} }
};
Capture::Capture(HWND hwnd) : impl(std::make_shared<Impl>(hwnd)) { impl->start(); }
Capture::~Capture() { try { impl->stop(); } catch (...) {} }
Image Capture::frame(unsigned timeout_ms) {
    std::unique_lock lock(impl->mutex);
    impl->changed.wait_for(lock,std::chrono::milliseconds(timeout_ms),[&] { return impl->closed || !impl->error.empty() || impl->pending || !impl->latest.pixels.empty(); });
    if (impl->closed || !IsWindow(impl->hwnd) || IsIconic(impl->hwnd) || !IsWindowVisible(impl->hwnd)) throw std::runtime_error("Capture target unavailable");
    if (!impl->error.empty()) throw std::runtime_error(impl->error);
    impl->read_pixels();
    if (impl->latest.pixels.empty()) throw std::runtime_error("Capture frame timeout");
    RECT client{}, bounds{};
    if (!GetClientRect(impl->hwnd,&client)) throw std::runtime_error("Client rectangle unavailable");
    POINT origin{0,0};
    if (!ClientToScreen(impl->hwnd,&origin)) throw std::runtime_error("Client origin unavailable");
    check_hresult(DwmGetWindowAttribute(impl->hwnd,DWMWA_EXTENDED_FRAME_BOUNDS,&bounds,sizeof bounds));
    if (bounds.right-bounds.left!=impl->latest.w || bounds.bottom-bounds.top!=impl->latest.h) {
        if (!GetWindowRect(impl->hwnd,&bounds) || bounds.right-bounds.left!=impl->latest.w || bounds.bottom-bounds.top!=impl->latest.h)
            throw std::runtime_error("Capture/client geometry changed; waiting for a fresh frame");
    }
    return impl->latest.crop({origin.x-bounds.left,origin.y-bounds.top,client.right-client.left,client.bottom-client.top});
}
