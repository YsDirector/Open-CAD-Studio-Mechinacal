#!/usr/bin/env python3
"""本机私有词表加载器：私有名称只存在**仓库之外**，代码里不写死任何私有串。

两份词表都放本机配置目录（默认 `~/.config/ocsm/`，可用 `OCSM_CONFIG_DIR` 改），
每行一个词，`#` 起注释：

* `junk-names.txt` —— 历史上随图纸/模板带过来的旧环境残留名称。
  通用件守卫（`tools/frame_clean.py`、`tools/bom_template.py`）把它与下面的**通用词**合并使用；
  环境变量 `OCSM_JUNK_NAMES`（逗号或换行分隔）可另行补充。
  **两份都没有 ⇒ 只用通用词**，不报错、也不改变清洗/生成流程本身。
* `hygiene-words.txt` —— 发布前卫生扫描的个人/旧环境标识词（`tools/hygiene_scan.py` 读它）。

原则：
* 通用软件名（ZWCAD、天河 PCCAD 的样式/表前缀等）是公开信息，写在 `GENERIC_JUNK` 里，可以随仓走；
* 私有名称（前雇主名、私人模板名…）**只允许**出现在本机词表，仓里、脚本里、文档里都不写。
"""
from __future__ import annotations

import os
from pathlib import Path

CONFIG_DIR = Path(os.environ.get("OCSM_CONFIG_DIR") or Path.home() / ".config/ocsm")
JUNK_NAMES_FILE = CONFIG_DIR / "junk-names.txt"
HYGIENE_WORDS_FILE = CONFIG_DIR / "hygiene-words.txt"
JUNK_NAMES_ENV = "OCSM_JUNK_NAMES"

# 通用残留词：来自公开软件/模板（ZWCAD 的 Zmw* 样式库、天河 PCCAD 的 TH_* 表）
GENERIC_JUNK = ("Zwm", "ZWM", "PCCAD", "TH_Paper", "TH_CSL")


def read_word_file(path: Path) -> list[str]:
    """读词表：每行一个词，`#` 起注释；文件不存在 ⇒ 空表（不报错）。"""
    try:
        raw = path.read_text(encoding="utf-8")
    except OSError:
        return []
    out: list[str] = []
    for line in raw.splitlines():
        word = line.split("#", 1)[0].strip()
        if word and word not in out:
            out.append(word)
    return out


def split_words(value: str) -> list[str]:
    """把 `OCSM_JUNK_NAMES` 这类值拆成词（逗号 / 换行 / 分号都行），去重保序。"""
    out: list[str] = []
    for chunk in value.replace(",", "\n").replace(";", "\n").splitlines():
        word = chunk.strip()
        if word and word not in out:
            out.append(word)
    return out


def private_names(path: Path | None = None, env: str = JUNK_NAMES_ENV) -> tuple[str, ...]:
    """本机私有的旧环境残留名：环境变量优先，其次词表文件；都没有 ⇒ 空元组。"""
    words = split_words(os.environ.get(env, ""))
    for word in read_word_file(path or JUNK_NAMES_FILE):
        if word not in words:
            words.append(word)
    return tuple(words)


def junk_names(path: Path | None = None, env: str = JUNK_NAMES_ENV) -> tuple[str, ...]:
    """通用件守卫的完整检查集合 = 本机私有名 + 通用词（去重、顺序稳定）。"""
    words = list(private_names(path, env))
    for word in GENERIC_JUNK:
        if word not in words:
            words.append(word)
    return tuple(words)


def hygiene_words(path: Path | None = None) -> tuple[str, ...]:
    """发布前卫生扫描的词表（个人标识 / 旧环境名）。"""
    return tuple(read_word_file(path or HYGIENE_WORDS_FILE))
