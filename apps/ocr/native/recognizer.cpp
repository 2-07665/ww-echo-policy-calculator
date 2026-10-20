// PP-OCRv6 recognition only: BGR / 127.5 - 1, CHW, height 48, CTC.
// Configuration is copied from UmaOCR's pinned PaddlePaddle model, not its game heuristics.
#include "ocr.hpp"
#include <onnxruntime_cxx_api.h>
#include <algorithm>
#include <array>
#include <cmath>
#include <fstream>
#include <set>
#include <sstream>
#include <stdexcept>

struct Recognizer::Impl {
    Ort::Env env{ORT_LOGGING_LEVEL_WARNING, "echo-ocr"};
    Ort::SessionOptions options;
    Ort::Session session{nullptr};
    std::vector<std::string> characters{""};
    std::string input, output;
    explicit Impl(const std::filesystem::path& directory) {
        if (sha256(directory / "inference.onnx") != "5435fd747c9e0efe15a96d0b378d5bd157e9492ed8fd80edf08f30d02fa24634" ||
            sha256(directory / "dictionary.txt") != "b5f2bfe2bdd9448429e3e82b51c789775d9b42f2403d082b00662eb77e401c5d")
            throw std::runtime_error("OCR model/dictionary checksum mismatch; run prepare-ocr.ps1");
        std::ifstream file(directory / "dictionary.txt");
        std::string line;
        while (std::getline(file, line)) { if (!line.empty() && line.back() == '\r') line.pop_back(); characters.push_back(line); }
        characters.push_back(" ");
        options.SetIntraOpNumThreads(4);
        options.SetInterOpNumThreads(1);
        options.AddConfigEntry("session.intra_op.allow_spinning", "0");
        options.AddConfigEntry("session.inter_op.allow_spinning", "0");
        options.SetGraphOptimizationLevel(GraphOptimizationLevel::ORT_ENABLE_ALL);
        session = Ort::Session(env, (directory / "inference.onnx").c_str(), options);
        Ort::AllocatorWithDefaultOptions allocator;
        input = session.GetInputNameAllocated(0, allocator).get();
        output = session.GetOutputNameAllocated(0, allocator).get();
    }
};
Recognizer::Recognizer(const std::filesystem::path& directory) : impl(std::make_unique<Impl>(directory)) {}
Recognizer::~Recognizer() = default;
Text Recognizer::read(const Image& image, Rect r) {
    // Canonical rectangles are always relative to the 1920x1080 game client.
    Rect actual{(int)std::lround(r.x * image.w / 1920.0), (int)std::lround(r.y * image.h / 1080.0),
                (int)std::lround(r.w * image.w / 1920.0), (int)std::lround(r.h * image.h / 1080.0)};
    Image crop = image.crop(actual);
    int resized = (int)std::ceil(48.0 * crop.w / crop.h);
    int width = std::max(320, resized);
    if (width > 2048) throw std::runtime_error("OCR crop too wide");
    std::vector<float> tensor((size_t)3*48*width, 0.0f);
    for (int y = 0; y < 48; ++y) {
        double fy = std::clamp((y+.5)*crop.h/48.0-.5, 0.0, (double)crop.h-1);
        int y0 = (int)fy, y1 = std::min(y0+1,crop.h-1);
        for (int x = 0; x < resized; ++x) {
            double fx = std::clamp((x+.5)*crop.w/resized-.5, 0.0, (double)crop.w-1);
            int x0=(int)fx, x1=std::min(x0+1,crop.w-1);
            for (int c = 0; c < 3; ++c) {
                auto pixel = [&](int xx, int yy) { return crop.pixels[((size_t)yy*crop.w+xx)*4+c]; };
                double top=pixel(x0,y0)*(1-(fx-x0))+pixel(x1,y0)*(fx-x0);
                double bottom=pixel(x0,y1)*(1-(fx-x0))+pixel(x1,y1)*(fx-x0);
                tensor[((size_t)c*48+y)*width+x]=(float)((top*(1-(fy-y0))+bottom*(fy-y0))/127.5-1);
            }
        }
    }
    std::array<int64_t,4> shape{1,3,48,width};
    auto memory = Ort::MemoryInfo::CreateCpu(OrtArenaAllocator, OrtMemTypeDefault);
    auto value = Ort::Value::CreateTensor<float>(memory,tensor.data(),tensor.size(),shape.data(),shape.size());
    const char* inputs[]{impl->input.c_str()}; const char* outputs[]{impl->output.c_str()};
    auto results=impl->session.Run(Ort::RunOptions{nullptr},inputs,&value,1,outputs,1);
    auto dims=results.at(0).GetTensorTypeAndShapeInfo().GetShape();
    if (dims.size()!=3 || dims[0]!=1 || dims[2]!=(int64_t)impl->characters.size())
        throw std::runtime_error("OCR model output/dictionary mismatch");
    const float* data=results[0].GetTensorData<float>();
    Text result;
    int previous=-1, count=0;
    for (int64_t t=0;t<dims[1];++t) {
        const float* start=data+t*dims[2];
        int index=(int)(std::max_element(start,start+dims[2])-start);
        if (index && index!=previous) { result.text+=impl->characters[index]; result.confidence+=start[index]; ++count; }
        previous=index;
    }
    if (count) result.confidence/=count;
    return result;
}
static void replace_all(std::string& s, const std::string& from, const std::string& to) {
    size_t pos=0;
    while ((pos=s.find(from,pos))!=std::string::npos) { s.replace(pos,from.size(),to); pos+=to.size(); }
}
static std::string normalize(std::string s) {
    for (auto c : {" ","\t","\r","\n","　"}) replace_all(s,c,"");
    replace_all(s,"％","%");
    return s;
}
static std::string label(std::string s) {
    s=normalize(s);
    // Particle effects beside a fixed text crop can decode as terminal punctuation.
    // Only remove punctuation after labels, never edit digits or guess a stat name.
    bool changed=true;
    while (changed) {
        changed=false;
        for (std::string suffix : {"。","、","."}) {
            if (s.size()>=suffix.size() && s.ends_with(suffix)) { s.resize(s.size()-suffix.size()); changed=true; }
        }
    }
    return s;
}
bool parse_entry(std::string name, std::string value, Entry& entry) {
    name=label(name); value=normalize(value);
    replace_all(value,",",".");
    static const std::vector<std::pair<std::string,std::string>> names{
        {"暴击伤害","Crit_Damage"},{"暴击","Crit_Rate"},{"共鸣效率","ER"},
        {"普攻伤害加成","Basic_Attack_Damage"},{"重击伤害加成","Heavy_Attack_Damage"},
        {"共鸣技能伤害加成","Skill_Damage"},{"共鸣解放伤害加成","Ult_Damage"},
        {"攻击","Attack"},{"生命","HP"},{"防御","Defence"}};
    auto found=std::find_if(names.begin(),names.end(),[&](auto& item){ return name==item.first; });
    if (found==names.end() || value.empty()) return false;
    bool percent=value.back()=='%';
    if (percent) value.pop_back();
    // Fixed stats are integral. Percentage stats must explicitly retain the percent sign.
    if (value.empty() || value.find_first_not_of("0123456789.")!=std::string::npos) return false;
    auto dot=value.find('.');
    if (dot!=std::string::npos && (!percent || dot==0 || value.size()-dot!=2)) return false;
    std::string digits=value; replace_all(digits,".","");
    if (digits.size()>5) return false;
    int number=std::stoi(digits);
    if (percent && dot==std::string::npos) number*=10;
    entry={found->second,number};
    if (entry.name=="Attack" || entry.name=="Defence" || entry.name=="HP") {
        if (!percent) entry.name+="_Flat";
    } else if (!percent) return false;
    static const std::vector<int> common{64,71,79,86,94,101,109,116};
    const std::vector<int>* options=&common;
    static const std::vector<int> crit{63,69,75,81,87,93,99,105}, damage{126,138,150,162,174,186,198,210},
        defence{81,90,100,109,118,128,138,147}, attackFlat{30,40,50,60}, defenceFlat{40,50,60,70},
        hpFlat{320,360,390,430,470,510,540,580}, er{68,76,84,92,100,108,116,124};
    if (entry.name=="Crit_Rate") options=&crit;
    else if (entry.name=="Crit_Damage") options=&damage;
    else if (entry.name=="Defence") options=&defence;
    else if (entry.name=="Attack_Flat") options=&attackFlat;
    else if (entry.name=="Defence_Flat") options=&defenceFlat;
    else if (entry.name=="HP_Flat") options=&hpFlat;
    else if (entry.name=="ER") options=&er;
    return std::find(options->begin(),options->end(),number)!=options->end();
}
std::string Scan::packet() const {
    std::string result="{\"buffEntries\":[";
    for (size_t i=0;i<entries.size();++i) {
        if (i) result+=",";
        result+="{\"buffName\":"+quote(entries[i].name)+",\"buffValue\":"+std::to_string(entries[i].value)+"}";
    }
    return result+"]}";
}
Scan Recognizer::scan(const Image& image) {
    Scan scan;
    if (image.w<960 || image.h<540 || std::abs((double)image.w/image.h-16.0/9)>0.015) {
        scan.reason="unsupported_size"; return scan;
    }
    auto text=[&](Rect r) { auto t=read(image,r); scan.raw.push_back(t); return t; };
    auto header=text({108,50,130,40});
    if (normalize(header.text)=="声骸强化" && header.confidence>=.85f) scan.scene=2;
    else {
        auto cost=text({1504,186,100,33});
        auto button=text({1534,976,105,39});
        auto label=normalize(button.text);
        if (normalize(cost.text).find("COST")!=std::string::npos && cost.confidence>=.85f &&
            (label=="装配" || label=="卸下" || label=="替换") && button.confidence>=.85f) scan.scene=1;
    }
    if (!scan.scene) { scan.reason="not_echo_page"; return scan; }
    const int rightY[]{308,343,378,413,448};
    const int leftY[]{344,387,430,473,516};
    std::set<std::string> used;
    for (int i=0;i<5;++i) {
        auto name=text(scan.scene==1 ? Rect{1540,rightY[i],208,31} : Rect{252,leftY[i],225,33});
        auto normalized=normalize(name.text);
        // In the equipment scene the skill section follows the last revealed substat.
        // In the upgrade scene the remaining slots explicitly show the lock message.
        if (scan.scene==2 && name.confidence>=.85f && normalized.find("强化至+")==0) break;
        auto value=text(scan.scene==1 ? Rect{1752,rightY[i],105,31} : Rect{560,leftY[i],103,33});
        Entry entry;
        if (name.confidence<.85f || value.confidence<.85f || !parse_entry(name.text,value.text,entry) || !used.insert(entry.name).second) {
            if (scan.scene==1) {
                auto section=text({1503,rightY[i]+5,210,33});
                if (section.confidence>=.85f && label(section.text)=="声骸技能") break;
            }
            scan.reason="uncertain_row_"+std::to_string(i+1); return scan;
        }
        scan.entries.push_back(entry);
    }
    scan.valid=!scan.entries.empty();
    scan.reason=scan.valid ? "recognized" : "no_substats";
    return scan;
}
