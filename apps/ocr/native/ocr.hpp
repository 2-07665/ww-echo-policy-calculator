#pragma once
#include <windows.h>
#include <filesystem>
#include <memory>
#include <string>
#include <vector>

struct Rect { int x, y, w, h; };
struct Image {
    int w = 0, h = 0;
    std::vector<unsigned char> pixels; // tightly packed BGRA, client coordinates
    Image crop(Rect r) const;
    static Image load(const std::filesystem::path& path);
    void save(const std::filesystem::path& path) const;
};
std::string utf8(const std::wstring& text);
std::string quote(const std::string& text);
void status(const char* code, const std::string& detail = "");
std::string sha256(const std::filesystem::path& path);

struct Text { std::string text; float confidence = 0; };
struct Entry { std::string name; int value; bool operator==(const Entry&) const = default; };
struct Scan {
    int scene = 0;
    bool valid = false;
    std::string reason;
    std::vector<Entry> entries;
    std::vector<Text> raw;
    std::string packet() const;
};
bool parse_entry(std::string name, std::string value, Entry& entry);
bool resolve_entry(const std::vector<Text>& names, const std::vector<Text>& values, Entry& entry);
// One changed frame cannot replace the last confirmed result. Invalid capture
// resets both confirmation and deduplication so recovery can refill the UI.
class Confirmation {
    std::string previous, sent;
public:
    void clear() { previous.clear(); sent.clear(); }
    bool observe(const std::string& key) {
        bool deliver=key==previous && key!=sent;
        if (deliver) sent=key;
        previous=key;
        return deliver;
    }
    bool confirmed(const std::string& key) const { return key==sent; }
};
class Recognizer {
    struct Impl;
    std::unique_ptr<Impl> impl;
public:
    explicit Recognizer(const std::filesystem::path& directory);
    ~Recognizer();
    Text read(const Image& image, Rect canonical);
    Scan scan(const Image& image);
};
struct Window { HWND handle; std::wstring title; };
bool is_game_title(std::wstring title);
std::vector<Window> game_windows();
class Capture {
    struct Impl;
    std::shared_ptr<Impl> impl;
public:
    explicit Capture(HWND window);
    ~Capture();
    Image frame(unsigned timeout_ms = 2500);
};
