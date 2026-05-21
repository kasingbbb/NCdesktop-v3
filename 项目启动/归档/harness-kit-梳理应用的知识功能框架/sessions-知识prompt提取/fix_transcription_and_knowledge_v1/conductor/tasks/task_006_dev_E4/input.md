# Task 006 — Dev E4（自动触发链路 F-7）

MVP 简化：scheduler 物化成功后发射 `notecapt/concept-extract-requested` 事件（libraryId / triggerAssetId），前端监听后调 `extract_concepts_for_library(force=false)`。F-8 的 `concepts_extraction_log` 去重确保不会无限触发。
