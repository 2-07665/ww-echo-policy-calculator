#include <winsock2.h>
#include "ocr.hpp"
#include <winrt/base.h>
#include <atomic>
#include <chrono>
#include <iostream>
#include <thread>
#include <stdexcept>

static std::atomic<bool> stopping=false;
static BOOL WINAPI control(DWORD) { stopping=true; return TRUE; }
static std::filesystem::path executable_directory() {
    wchar_t path[32768];
    DWORD length=GetModuleFileNameW(nullptr,path,32768);
    if (!length || length==32768) throw std::runtime_error("Executable path unavailable");
    return std::filesystem::path(std::wstring(path,length)).parent_path();
}
static void emit_scan(const Scan& scan) {
    std::cout << "{\"scene\":" << scan.scene << ",\"valid\":" << (scan.valid ? "true":"false")
              << ",\"reason\":" << quote(scan.reason) << ",\"packet\":" << scan.packet() << ",\"raw\":[";
    for (size_t i=0;i<scan.raw.size();++i) {
        if (i) std::cout << ',';
        std::cout << "{\"text\":" << quote(scan.raw[i].text) << ",\"confidence\":" << scan.raw[i].confidence << '}';
    }
    std::cout << "]}" << std::endl;
}
static void require(bool condition) { if (!condition) throw std::runtime_error("OCR self-test failed"); }
static void self_test() {
    require(is_game_title(L"鸣潮  "));
    require(is_game_title(L"\t鸣潮\u3000"));
    require(is_game_title(L" Wuthering Waves "));
    require(!is_game_title(L"鸣潮启动器"));
    require(!is_game_title(L" \t"));
    Entry entry;
    require(parse_entry("暴击","6.3%",entry) && entry==Entry{"Crit_Rate",63});
    require(parse_entry("生命","430",entry) && entry==Entry{"HP_Flat",430});
    require(parse_entry("防御","8.1%",entry) && entry==Entry{"Defence",81});
    require(parse_entry("共鸣解放伤害加成","7.9％",entry) && entry==Entry{"Ult_Damage",79});
    for (auto value : {"6.4%","63","6.3","6.3.0%","-6.3%","6.30%","6O%","999999999%"}) require(!parse_entry("暴击",value,entry));
    require(!parse_entry("攻击","150",entry)); // main stat cannot pass
    require(!parse_entry("声骸技能","6.4%",entry));
    require(resolve_entry({{"生命·",.79f},{"生命",.99f}},{{"8.6%",.99f}},entry) && entry==Entry{"HP",86});
    require(!resolve_entry({{"生命",.84f}},{{"8.6%",.99f}},entry));
    require(!resolve_entry({{"生命",.99f}},{{"8.6%",.84f}},entry));
    require(!resolve_entry({{"生命",.99f}},{{"8.7%",.99f}},entry));
    require(!resolve_entry({{"暴击",.99f}},{{"6.3%",.99f},{"6.9%",.99f}},entry));
    require(!resolve_entry({{"生命",.99f},{"攻击",.99f}},{{"8.6%",.99f}},entry));
    Confirmation confirmation;
    require(!confirmation.observe("scene2:1920:1080:two"));
    require(confirmation.observe("scene2:1920:1080:two"));
    require(!confirmation.observe("scene2:1920:1080:three"));
    require(!confirmation.observe("scene2:1920:1080:two")); // transient extra row
    require(!confirmation.observe("scene2:1920:1080:three"));
    require(confirmation.observe("scene2:1920:1080:three"));
    require(!confirmation.observe("scene2:1920:1080:three")); // no duplicate
    require(!confirmation.observe("scene2:1280:720:three")); // resize resets agreement
    confirmation.clear(); // rejected frame / minimize / page change
    require(!confirmation.observe("scene2:1280:720:three"));
    require(confirmation.observe("scene2:1280:720:three"));
    status("self_test_passed");
}
int wmain(int argc,wchar_t** argv) {
    SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    SetConsoleOutputCP(CP_UTF8);
    SetConsoleCtrlHandler(control,TRUE);
    try {
        winrt::init_apartment(winrt::apartment_type::multi_threaded);
        std::filesystem::path directory=executable_directory(), imagePath, captureOutput;
        HWND specified=nullptr;
        Rect client{};
        bool list=false, test=false;
        for (int i=1;i<argc;++i) {
            std::wstring arg=argv[i];
            auto next=[&]()->std::wstring { if (++i>=argc) throw std::runtime_error("Missing argument"); return argv[i]; };
            if (arg==L"--image") imagePath=next();
            else if (arg==L"--models") directory=next();
            else if (arg==L"--hwnd") specified=(HWND)(uintptr_t)std::stoull(next());
            else if (arg==L"--capture-output") captureOutput=next();
            else if (arg==L"--client-rect") { auto s=next(); if (swscanf_s(s.c_str(),L"%d,%d,%d,%d",&client.x,&client.y,&client.w,&client.h)!=4) throw std::runtime_error("Invalid client rect"); }
            else if (arg==L"--windows") list=true;
            else if (arg==L"--self-test") test=true;
            else throw std::runtime_error("Unknown OCR argument");
        }
        if (test) { self_test(); return 0; }
        if (list) {
            std::cout << '[';
            auto windows=game_windows();
            for (size_t i=0;i<windows.size();++i) {
                if (i) std::cout << ',';
                std::cout << "{\"id\":" << quote(std::to_string((uintptr_t)windows[i].handle)) << ",\"title\":" << quote(utf8(windows[i].title)) << '}';
            }
            std::cout << ']' << std::endl; return 0;
        }
        if (!captureOutput.empty()) {
            if (!specified) throw std::runtime_error("Capture test requires --hwnd");
            Capture capture(specified);
            capture.frame().save(captureOutput);
            status("capture_saved"); return 0;
        }
        Recognizer recognizer(directory);
        if (!imagePath.empty()) {
            auto image=Image::load(imagePath);
            if (client.w) image=image.crop(client);
            auto start=std::chrono::steady_clock::now();
            auto scan=recognizer.scan(image); emit_scan(scan);
            std::cerr << "scan_ms=" << std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now()-start).count() << std::endl;
            return scan.valid ? 0:2;
        }
        WSADATA data{};
        if (WSAStartup(MAKEWORD(2,2),&data)) throw std::runtime_error("UDP initialization failed");
        SOCKET sock=socket(AF_INET,SOCK_DGRAM,IPPROTO_UDP);
        if (sock==INVALID_SOCKET) throw std::runtime_error("UDP socket failed");
        sockaddr_in address{}; address.sin_family=AF_INET; address.sin_port=htons(9999); address.sin_addr.s_addr=htonl(INADDR_LOOPBACK);
        // Stdin belongs to the desktop parent. EOF stops the child after parent exit.
        std::thread([] { char c; while (std::cin.get(c)) { if (c=='q') break; } stopping=true; }).detach();
        status("ready");
        std::unique_ptr<Capture> capture;
        HWND target=nullptr;
        Confirmation confirmation;
        auto clear=[&] { confirmation.clear(); };
        auto nextScan=std::chrono::steady_clock::now();
        while (!stopping) {
            auto now=std::chrono::steady_clock::now();
            if (now<nextScan) { std::this_thread::sleep_for(std::chrono::milliseconds(50)); continue; }
            nextScan=now+std::chrono::milliseconds(300);
            HWND hwnd=specified;
            if (!hwnd) {
                auto windows=game_windows();
                if (windows.size()!=1) {
                    capture.reset(); target=nullptr; clear();
                    status(windows.empty()?"waiting_game":"select_window"); continue;
                }
                hwnd=windows[0].handle;
            }
            if (!IsWindow(hwnd) || !IsWindowVisible(hwnd) || IsIconic(hwnd)) {
                capture.reset(); target=nullptr; clear(); status("paused"); continue;
            }
            try {
                if (!capture || hwnd!=target) { capture=std::make_unique<Capture>(hwnd); target=hwnd; clear(); }
                auto image=capture->frame();
                auto scan=recognizer.scan(image);
                if (!scan.valid) { clear(); status(scan.reason.c_str()); continue; }
                std::string packet=scan.packet();
                // Confirm page, client geometry and substats; item-name OCR noise
                // must not restart confirmation of otherwise identical substats.
                std::string key=std::to_string(scan.scene)+":"+std::to_string(image.w)+":"+std::to_string(image.h)+":"+packet;
                if (confirmation.observe(key)) {
                    if (sendto(sock,packet.data(),(int)packet.size(),0,(sockaddr*)&address,sizeof address)==SOCKET_ERROR)
                        throw std::runtime_error("UDP delivery failed");
                }
                status(confirmation.confirmed(key) ? "recognized":"confirming");
            } catch (const winrt::hresult_error& e) { clear(); capture.reset(); status("capture_error",winrt::to_string(e.message())); }
              catch (const std::exception& e) { clear(); capture.reset(); status("capture_error",e.what()); }
        }
        closesocket(sock); WSACleanup(); return 0;
    } catch (const winrt::hresult_error& e) { status("error",winrt::to_string(e.message())); return 1; }
      catch (const std::exception& e) { status("error",e.what()); return 1; }
}
