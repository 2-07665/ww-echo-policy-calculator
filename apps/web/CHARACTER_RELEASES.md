# Character order

Checked on 2026-10-19. `preset-data.js` records **first playable availability**, not
rerun dates, resource IDs or the order in a downloaded template file. Every scoring
system and global search use this same descending date order. Same-day characters
retain their original relative order; variants retain their source order within a
character. Generic presets come after dated characters, and custom presets remain
in their own group. Sorting never changes a preset's stored ID.

Sources:

- [Huiji Wiki version timeline](https://wuwa.huijiwiki.com/wiki/1.0%E7%89%88%E6%9C%AC): first event banners from 1.0 through 3.7, including four-star debuts for Youhu and Lumi.
- [Fandom's release-date category](https://wutheringwaves.fandom.com/wiki/Category%3AResonators_by_Release_Date): independent cross-check for the earlier character order.
- [Buling](https://wuwa.huijiwiki.com/wiki/%E5%8D%9C%E7%81%B5): 2025-12-11, after Chisa (2025-11-20).
- [Luuk Herssen](https://wuwa.huijiwiki.com/wiki/%E9%99%86%C2%B7%E8%B5%AB%E6%96%AF): 2026-02-26. The version overview's “2025” year for 3.1 is a typo; the character page and [3.1 announcement](https://wuwa.huijiwiki.com/wiki/3.1%E7%89%88%E6%9C%AC) establish 2026.
- [Rover](https://wuwa.huijiwiki.com/wiki/%E6%BC%82%E6%B3%8A%E8%80%85): Spectro/Havoc 2024-05-23, Aero 2025-03-27, Electro 2026-07-10. Each element is ordered by its own availability.
- [Yangyang: Xuanling](https://wuwa.huijiwiki.com/wiki/%E7%A7%A7%E7%A7%A7%C2%B7%E7%8E%84%E7%BF%8E): 2026-07-10; distinct from the launch character.
- [Qingxiao](https://wuwa.huijiwiki.com/wiki/%E6%B8%85%E5%AE%B5): 2026-08-20.
- [Jingran](https://wuwa.huijiwiki.com/wiki/%E6%99%AF%E7%87%83): 2026-09-10.
- [Hsin](https://wuwa.huijiwiki.com/wiki/%E5%BF%83): 2026-09-30.
- [3.7 announcement](https://wuwa.huijiwiki.com/wiki/3.7%E7%89%88%E6%9C%AC%E3%80%8C%E9%95%9C%E9%94%81%E5%A6%84%E4%B8%96%EF%BC%8C%E5%BF%83%E7%85%A7%E7%BA%A2%E5%B0%98%E3%80%8D): Suoming is scheduled for 2026-10-22. The upstream preset is retained at the top and labeled “未实装 / Upcoming” before that date.

Legacy aliases 雪豹/凌阳, 赤霞/炽霞, 桃祁/桃祈 and 坎特雷拉/坎特蕾拉 share dates.
The UI has separate **preset** and **variant** selects: XutheringWavesUID presents
61 preset groups, with 278 variants across those groups. The new snapshot replaces
the former QQ Bot snapshot; neither old weights nor duplicate old entries are appended.
