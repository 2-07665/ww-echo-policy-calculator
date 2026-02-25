import re
import socket
import json
from qfluentwidgets import FluentIcon
from ok import Logger, TaskDisabledException
from src.task.BaseWWTask import BaseWWTask

logger = Logger.get_logger(__name__)

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)

number_pattern = re.compile(r"^[\d.%]+$")
property_pattern = re.compile(r"^\D*$")

BUFF_VALUE_OPTIONS = {
    "Crit_Rate": {63, 69, 75, 81, 87, 93, 99, 105},
    "Crit_Damage": {126, 138, 150, 162, 174, 186, 198, 210},
    "Attack": {64, 71, 79, 86, 94, 101, 109, 116},
    "Defence": {81, 90, 100, 109, 118, 128, 138, 147},
    "HP": {64, 71, 79, 86, 94, 101, 109, 116},
    "Attack_Flat": {30, 40, 50, 60},
    "Defence_Flat": {40, 50, 60, 70},
    "HP_Flat": {320, 360, 390, 430, 470, 510, 540, 580},
    "ER": {68, 76, 84, 92, 100, 108, 116, 124},
    "Basic_Attack_Damage": {64, 71, 79, 86, 94, 101, 109, 116},
    "Heavy_Attack_Damage": {64, 71, 79, 86, 94, 101, 109, 116},
    "Skill_Damage": {64, 71, 79, 86, 94, 101, 109, 116},
    "Ult_Damage": {64, 71, 79, 86, 94, 101, 109, 116},
}

class EchoOCRTask(BaseWWTask):

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self.name = "Echo OCR"
        self.description = "OCR识别声骸副词条"
        self.group_name = "My"
        self.icon = FluentIcon.ALBUM
        self.default_config = {"识别间隔(s)": 2.0, "端口": 9999}
        self._is_echo_page = False
        self._is_echo_upgrade_page = False
        self._last_sent_message = None
        self._max_pairing_y_diff = 10

    def run(self):
        scan_interval = float(self.config.get("识别间隔(s)", 1.0))
        port = int(self.config.get("端口", 9999))

        self.info_set("在声骸装备界面", "否")
        self.info_set("在声骸强化界面", "否")

        try:
            while self._is_task_active():
                self.sleep(scan_interval)
                self._check_echo_page()
                self._check_echo_upgrade_page()
                if self._is_echo_page:
                    texts = self.read_echo_stat([0.74, 0.29, 0.96, 0.45])
                    self.send_echo_stat(texts, "127.0.0.1", port)
                elif self._is_echo_upgrade_page:
                    texts = self.read_echo_stat([0.11, 0.29, 0.36, 0.51])
                    self.send_echo_stat(texts, "127.0.0.1", port)
                

        except TaskDisabledException:
            logger.info("Echo OCR task stopped manually")

    def _is_task_active(self):
        return bool(getattr(self, "enabled", True) and getattr(self, "running", True))

    def _check_echo_page(self):
        page_detect = self.ocr(0.74, 0.90, 0.84, 0.95, match=["卸下","装配", "替换"])
        if page_detect and not self._is_echo_page:
            self._is_echo_page = True
            self.info_set("在声骸装备界面", "是")
        if not page_detect and self._is_echo_page:
            self._is_echo_page = False
            self.info_set("在声骸装备界面", "否")

    def _check_echo_upgrade_page(self):
        page_detect = self.ocr(0.0, 0.0, 0.15, 0.10, match=re.compile(r"^声\D强化$"))
        if page_detect and not self._is_echo_upgrade_page:
            self._is_echo_upgrade_page = True
            self.info_set("在声骸强化界面", "是")
        if not page_detect and self._is_echo_upgrade_page:
            self._is_echo_upgrade_page = False
            self.info_set("在声骸强化界面", "否")

    def read_echo_stat(self, coordinates):
        return self.ocr(*coordinates)

    def send_echo_stat(self, texts, host, port):
        buffs = self.find_boxes(texts, match=property_pattern)
        values = self.find_boxes(texts, match=number_pattern)

        buff_entries = self._build_buff_entries(buffs, values)
        if not buff_entries:
            return
        payload = {"buffEntries": buff_entries}
        message = json.dumps(payload, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
        is_different = message != self._last_sent_message
        sock.sendto(message, (host, port))
        if is_different:
            logger.info(f"Sent new OCR result {payload}")
        self._last_sent_message = message

    def _build_buff_entries(self, buffs, values):
        if not buffs or not values:
            return []

        sorted_buffs = sorted(buffs, key=lambda b: (b.y, b.x))
        unmatched_values = sorted(values, key=lambda b: (b.y, b.x))
        entries = []
        used_names = set()

        for buff in sorted_buffs:
            if not unmatched_values:
                break

            # The box coordinates depend on resoluion.
            # In 1080p, the delta y between rows is ~40, while the delta y between pair usually <= 3.
            candidate_values = [v for v in unmatched_values if abs(buff.y - v.y) <= self._max_pairing_y_diff]
            if not candidate_values:
                logger.debug(
                    f"Ignored unpaired buff '{buff.name}', no value within y diff <= {self._max_pairing_y_diff}"
                )
                continue

            closest_val = min(candidate_values, key=lambda v: abs(buff.y - v.y))
            unmatched_values.remove(closest_val)

            buff_name = self._to_buff_name(buff.name, closest_val.name)
            if not buff_name:
                logger.debug(f"Ignored unknown buff '{buff.name}'")
                continue
            if buff_name in used_names:
                logger.debug(f"Ignored duplicate buff '{buff_name}'")
                continue

            buff_value = self._to_buff_value(closest_val.name)
            if buff_value is None:
                logger.debug(f"Ignored invalid value '{closest_val.name}'")
                continue
            if buff_value not in BUFF_VALUE_OPTIONS[buff_name]:
                logger.debug(
                    f"Ignored out-of-range pair '{buff.name}={closest_val.name}' -> "
                    f"{buff_name}={buff_value}"
                )
                continue

            entries.append({"buffName": buff_name, "buffValue": buff_value})
            used_names.add(buff_name)
            if len(entries) >= 5:
                break

        return entries

    def _to_buff_name(self, raw_buff_name, raw_value):
        text = self._normalize_text(raw_buff_name)
        if not text:
            return None

        is_percent = "%" in self._normalize_value_text(raw_value)

        if "暴击伤害" in text:
            return "Crit_Damage"
        if "暴击" in text:
            return "Crit_Rate"
        if "攻击" in text:
            return "Attack" if is_percent else "Attack_Flat"
        if "效率" in text:
            return "ER"
        if "普攻" in text:
            return "Basic_Attack_Damage"
        if "重击" in text:
            return "Heavy_Attack_Damage"
        if "技能" in text:
            return "Skill_Damage"
        if "解放" in text:
            return "Ult_Damage"
        if "生命" in text:
            return "HP" if is_percent else "HP_Flat"
        if "防御" in text:
            return "Defence" if is_percent else "Defence_Flat"
        
        return None

    def _to_buff_value(self, raw_value):
        value_text = self._normalize_value_text(raw_value)
        if not value_text:
            return None
        try:
            if "%" in value_text:
                return int(round(float(value_text.rstrip("%")) * 10))
            return int(round(float(value_text)))
        except ValueError:
            return None

    def _normalize_text(self, text):
        return str(text).replace(" ", "").replace("\t", "").replace("\n", "")

    def _normalize_value_text(self, text):
        value = self._normalize_text(text)
        value = value.replace("％", "%").replace(",", ".")
        return value
