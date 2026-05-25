#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
extract_pdf_annotations.py
===========================

提取 PDF 用户标记（高亮 / 划线 / 删除线 / 波浪线 / 文字批注 / 手写）
并以 JSON 形式写到 stdout。

被 NCdesktop 的 Rust extractor 在 markitdown 转换成功后追加调用，
用于把 PDF 内的用户标记拼接到产出的 Markdown 末尾。

## 输出 schema（v1）

成功：
```json
{
  "version": 1,
  "page_count": 412,
  "annotations": [
    {
      "page": 24,
      "type": "Highlight",         // 见 SUPPORTED_TYPES
      "author": "加成",            // 已去除 UTF-16 BOM
      "comment": "",               // 用户写在标记上的批注（Contents 字段）
      "covered_text": "..."        // 高亮覆盖区域的原文（仅 QuadPoints 类标记）
    },
    ...
  ]
}
```

失败：
```json
{ "version": 1, "error": "类别:原因" }
```

## 约束

- 仅依赖 pdfplumber（已在 requirements.lock；transitively pulls pdfminer.six / pypdfium2 / Pillow）。
- 单个 annotation 解析失败不中断整体；只跳过并 log 到 stderr。
- 退出码：成功 0；致命错误（路径不存在/PDF 解析失败）非 0。
"""

from __future__ import annotations

import json
import sys
import traceback
from pathlib import Path
from typing import Any

# ─────────────────────────────────────────────────────────────────────────────
# 支持的 annotation 类型
# ─────────────────────────────────────────────────────────────────────────────
#
# QuadPoints 类：标记覆盖文字，需根据坐标到页面反查原文
QUAD_POINT_TYPES = {"Highlight", "Underline", "StrikeOut", "Squiggly"}
# Contents 类：用户输入的文字批注，直接读 Contents 字段
COMMENT_TYPES = {"Text", "FreeText"}
# 手写：无法转 MD，只记录"页 N 有手写"
INK_TYPES = {"Ink"}
SUPPORTED_TYPES = QUAD_POINT_TYPES | COMMENT_TYPES | INK_TYPES


def _decode_pdf_string(v: Any) -> str:
    """把 pdfplumber/pdfminer 返回的 annotation 字段值解码为 str。

    - bytes 可能是 UTF-16BE（带 BOM \\xfe\\xff）或 PDFDocEncoding/UTF-8；
    - pdfminer 的 PSLiteral / Name 对象有 .name；
    - 其余 fallback str()。
    """
    if v is None:
        return ""
    if isinstance(v, bytes):
        # PDF Spec §7.9.2.2: text string 可能以 BOM \xfe\xff 开头表示 UTF-16BE
        if v.startswith(b"\xfe\xff"):
            try:
                return v[2:].decode("utf-16-be", errors="replace").lstrip("﻿")
            except Exception:
                return v.decode("latin-1", errors="replace")
        # 否则当 UTF-8 试，失败回退 latin-1
        try:
            return v.decode("utf-8", errors="replace").lstrip("﻿")
        except Exception:
            return v.decode("latin-1", errors="replace")
    if hasattr(v, "name"):
        return str(v.name)
    return str(v).lstrip("﻿")


def _subtype_str(data: dict) -> str:
    """读 annotation 的 Subtype，标准化为不带斜杠的字符串（如 'Highlight'）。"""
    raw = data.get("Subtype")
    if raw is None:
        return ""
    if hasattr(raw, "name"):
        return raw.name
    return str(raw).lstrip("/")


def _extract_covered_text(page, quad_points) -> str:
    """根据 QuadPoints（8 元浮点组 × N quad）反查覆盖的原文。

    PDF Spec §12.5.6.10：QuadPoints 是 (x1,y1,x2,y2,x3,y3,x4,y4) per quad，
    四个点定义一个高亮四边形（通常等同矩形）。多个 quad 表示跨行高亮。
    pdfplumber 的坐标系 y 朝下，PDF 原生 y 朝上 —— 需翻转。
    """
    if not quad_points:
        return ""
    try:
        floats = [float(x) for x in quad_points]
    except (TypeError, ValueError):
        return ""

    page_height = float(page.height)
    parts: list[str] = []
    # 每 8 个数定义一个 quad
    for i in range(0, len(floats), 8):
        chunk = floats[i:i + 8]
        if len(chunk) < 8:
            continue
        xs = chunk[0::2]
        ys = chunk[1::2]
        x0, x1 = min(xs), max(xs)
        y0, y1 = min(ys), max(ys)
        # PDF → pdfplumber y 翻转
        top = page_height - y1
        bottom = page_height - y0
        try:
            cropped = page.crop((x0, top, x1, bottom))
            text = (cropped.extract_text() or "").strip()
            if text:
                parts.append(text)
        except Exception as e:
            # 单个 quad 失败不中断，记到 stderr
            print(f"  [warn] crop failed at quad {i//8}: {e}", file=sys.stderr)
    return " ".join(parts)


def extract(pdf_path: str) -> dict:
    """主入口：返回完整结果字典。"""
    import pdfplumber  # 延迟 import：错误时也能输出 JSON

    p = Path(pdf_path)
    if not p.exists():
        return {"version": 1, "error": f"file_not_found: {pdf_path}"}

    annotations: list[dict] = []
    page_count = 0

    with pdfplumber.open(pdf_path) as pdf:
        page_count = len(pdf.pages)
        for pg_idx, page in enumerate(pdf.pages):
            page_no = pg_idx + 1
            annots = page.annots or []
            for a in annots:
                try:
                    data = a.get("data") or {}
                    subtype = _subtype_str(data)
                    if subtype not in SUPPORTED_TYPES:
                        continue

                    entry: dict = {
                        "page": page_no,
                        "type": subtype,
                        "author": _decode_pdf_string(data.get("T")),
                        "comment": _decode_pdf_string(data.get("Contents")),
                    }

                    if subtype in QUAD_POINT_TYPES:
                        covered = _extract_covered_text(page, data.get("QuadPoints"))
                        entry["covered_text"] = covered

                    annotations.append(entry)
                except Exception as e:
                    print(
                        f"  [warn] annotation parse failed on page {page_no}: {e}",
                        file=sys.stderr,
                    )
                    continue

    return {
        "version": 1,
        "page_count": page_count,
        "annotations": annotations,
    }


def main(argv: list[str]) -> int:
    if len(argv) != 2:
        print(
            json.dumps({"version": 1, "error": "usage: extract_pdf_annotations.py <pdf-path>"}),
            flush=True,
        )
        return 2

    pdf_path = argv[1]
    try:
        result = extract(pdf_path)
    except Exception as e:
        print(
            json.dumps(
                {
                    "version": 1,
                    "error": f"unhandled_exception: {type(e).__name__}: {e}",
                },
                ensure_ascii=False,
            ),
            flush=True,
        )
        print(traceback.format_exc(), file=sys.stderr)
        return 1

    print(json.dumps(result, ensure_ascii=False), flush=True)
    return 0 if "error" not in result else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
