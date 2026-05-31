/**
 * task_017_frontmatter_renderer_dep — parseFrontmatter 单元测试
 *
 * 覆盖 AC-5 列出的 5 个 parse 场景：
 * - parseFrontmatter_extracts_yaml_and_body
 * - parseFrontmatter_handles_missing_frontmatter
 * - parseFrontmatter_returns_error_on_invalid_yaml
 * - parseFrontmatter_extracts_nc_and_kc_fields
 * - parseFrontmatter_yaml_load_safety
 */
import { describe, it, expect, vi, afterEach } from "vitest";
import { parseFrontmatter } from "../parseFrontmatter";

describe("parseFrontmatter", () => {
  it("parseFrontmatter_extracts_yaml_and_body — 提取 YAML 和正文", () => {
    const md = [
      "---",
      "ai_summary: hello",
      "ai_tags:",
      "  - foo",
      "  - bar",
      "---",
      "正文内容",
      "第二行",
    ].join("\n");

    const result = parseFrontmatter(md);
    expect(result.frontmatter).not.toBeNull();
    expect(result.frontmatter?.aiSummary).toBe("hello");
    expect(result.frontmatter?.aiTags).toEqual(["foo", "bar"]);
    expect(result.body).toBe("正文内容\n第二行");
    expect(result.parseError).toBeUndefined();
  });

  it("parseFrontmatter_handles_missing_frontmatter — 没有 frontmatter 时返回 null + 原文", () => {
    const md = "# 普通 markdown\n\n没有 frontmatter 头。";
    const result = parseFrontmatter(md);
    expect(result.frontmatter).toBeNull();
    expect(result.body).toBe(md);
    expect(result.parseError).toBeUndefined();
  });

  it("parseFrontmatter_handles_missing_frontmatter — 空字符串", () => {
    const result = parseFrontmatter("");
    expect(result.frontmatter).toBeNull();
    expect(result.body).toBe("");
  });

  it("parseFrontmatter_returns_error_on_invalid_yaml — 非法 YAML 时返回 parseError 不 throw", () => {
    // 未闭合的数组 — 必定 YAML 解析错误
    const md = ["---", "ai_tags: [unterminated", "---", "正文"].join("\n");
    const result = parseFrontmatter(md);
    expect(result.frontmatter).toBeNull();
    expect(result.parseError).toBeDefined();
    expect(result.parseError && result.parseError.length).toBeGreaterThan(0);
    // 解析失败时 body 回退到原 markdown，让上层能 fallback 渲染
    expect(result.body).toBe(md);
  });

  it("parseFrontmatter_extracts_nc_and_kc_fields — NC + KC 全字段 snake → camel 映射", () => {
    const md = [
      "---",
      "source_asset_id: 11111111-2222-3333-4444-555555555555",
      "derivative_version: 3",
      "extracted_at: 2026-05-27T08:00:00Z",
      "extractor_type: markitdown+kc",
      "quality_level: 3",
      "kc_doc_id: doc-abc12345",
      "kc_version: '0.9'",
      "kc_generated_at: 2026-05-27T07:59:50Z",
      "kc_tags_source: ai+rule",
      "kc_enriched: 'true'",
      "ai_tags:",
      "  - AI",
      "  - 机器学习",
      "rule_tags:",
      "  - AI",
      "  - ML",
      "ai_summary: 本文介绍了人工智能的基本概念",
      "ai_qa_pairs_count: 3",
      "paragraph_count: 7",
      "---",
      "正文",
    ].join("\n");

    const { frontmatter } = parseFrontmatter(md);
    expect(frontmatter).not.toBeNull();
    expect(frontmatter?.sourceAssetId).toBe("11111111-2222-3333-4444-555555555555");
    expect(frontmatter?.derivativeVersion).toBe(3);
    expect(frontmatter?.extractedAt).toBe("2026-05-27T08:00:00Z");
    expect(frontmatter?.extractorType).toBe("markitdown+kc");
    expect(frontmatter?.qualityLevel).toBe(3);
    expect(frontmatter?.kcDocId).toBe("doc-abc12345");
    expect(frontmatter?.kcVersion).toBe("0.9");
    expect(frontmatter?.kcGeneratedAt).toBe("2026-05-27T07:59:50Z");
    expect(frontmatter?.kcTagsSource).toBe("ai+rule");
    expect(frontmatter?.kcEnriched).toBe("true");
    expect(frontmatter?.aiTags).toEqual(["AI", "机器学习"]);
    expect(frontmatter?.ruleTags).toEqual(["AI", "ML"]);
    expect(frontmatter?.aiSummary).toBe("本文介绍了人工智能的基本概念");
    expect(frontmatter?.aiQaPairsCount).toBe(3);
    expect(frontmatter?.paragraphCount).toBe(7);
  });

  it("parseFrontmatter_yaml_load_safety — 拒绝危险 tag（!!python/object 等）", () => {
    // JSON_SCHEMA 不识别自定义 tag —— js-yaml 应抛错，被 parseFrontmatter 捕获为 parseError
    const md = [
      "---",
      "danger: !!python/object/apply:os.system ['rm -rf /']",
      "---",
      "正文",
    ].join("\n");

    const result = parseFrontmatter(md);
    expect(result.frontmatter).toBeNull();
    expect(result.parseError).toBeDefined();
    expect(result.parseError && result.parseError.toLowerCase()).toMatch(/tag|unknown|undefined/);
  });

  it("parseFrontmatter_yaml_load_safety — 拒绝 !!js/function tag", () => {
    const md = [
      "---",
      "evil: !!js/function 'function(){return 42}'",
      "---",
      "正文",
    ].join("\n");

    const result = parseFrontmatter(md);
    expect(result.frontmatter).toBeNull();
    expect(result.parseError).toBeDefined();
  });

  it("空 frontmatter（仅分隔符）→ null 但 body 正确", () => {
    const md = ["---", "---", "正文"].join("\n");
    const result = parseFrontmatter(md);
    expect(result.frontmatter).toBeNull();
    expect(result.body).toBe("正文");
    expect(result.parseError).toBeUndefined();
  });

  it("非法字段类型被静默丢弃，不污染输出", () => {
    // derivative_version 给字符串，应该不进入结果（白名单类型守卫）
    const md = [
      "---",
      "ai_summary: ok",
      "derivative_version: 'not-a-number'",
      "ai_tags:",
      "  - valid",
      "  - 123", // 数字混入数组 → 整个 aiTags 被丢弃（every 守卫）
      "---",
      "正文",
    ].join("\n");

    const { frontmatter } = parseFrontmatter(md);
    expect(frontmatter).not.toBeNull();
    expect(frontmatter?.aiSummary).toBe("ok");
    expect(frontmatter?.derivativeVersion).toBeUndefined();
    // aiTags 含数字混入 → 整数组丢弃
    expect(frontmatter?.aiTags).toBeUndefined();
  });

  describe("多块 frontmatter 兜底（剥离残留块）", () => {
    afterEach(() => {
      vi.restoreAllMocks();
    });

    it("双块输入 → 取第一块 frontmatter + 干净 body（不含残留 --- 块）", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const md = [
        "---",
        "ai_summary: 第一块",
        "ai_tags:",
        "  - foo",
        "---",
        "---",
        "ai_summary: 第二块应被丢弃",
        "---",
        "真正的正文",
        "第二行",
      ].join("\n");

      const result = parseFrontmatter(md);
      // 取首块
      expect(result.frontmatter).not.toBeNull();
      expect(result.frontmatter?.aiSummary).toBe("第一块");
      expect(result.frontmatter?.aiTags).toEqual(["foo"]);
      // body 干净，不含残留 frontmatter 块
      expect(result.body).toBe("真正的正文\n第二行");
      expect(result.body).not.toContain("---");
      expect(result.body).not.toContain("第二块");
      expect(result.parseError).toBeUndefined();
      // 残留块触发了 warn
      expect(warn).toHaveBeenCalled();
    });

    it("三块输入 → 取第一块 frontmatter + 剥离后两块", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const md = [
        "---",
        "ai_summary: 头块",
        "---",
        "---",
        "ai_summary: 中块",
        "---",
        "---",
        "ai_summary: 尾块",
        "---",
        "最终正文",
      ].join("\n");

      const result = parseFrontmatter(md);
      expect(result.frontmatter?.aiSummary).toBe("头块");
      expect(result.body).toBe("最终正文");
      expect(result.body).not.toContain("---");
      expect(result.parseError).toBeUndefined();
      expect(warn).toHaveBeenCalled();
    });

    it("单块输入 → 不退化（无残留剥离，不 warn）", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const md = ["---", "ai_summary: 唯一块", "---", "正文内容", "第二行"].join("\n");

      const result = parseFrontmatter(md);
      expect(result.frontmatter?.aiSummary).toBe("唯一块");
      expect(result.body).toBe("正文内容\n第二行");
      expect(warn).not.toHaveBeenCalled();
    });

    it("无 frontmatter → 不退化（原文 body，不 warn）", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const md = "# 普通 markdown\n\n中间有 --- 水平线\n\n---\n\n结尾";

      const result = parseFrontmatter(md);
      expect(result.frontmatter).toBeNull();
      expect(result.body).toBe(md);
      expect(warn).not.toHaveBeenCalled();
    });

    it("正文中的 --- 水平线不被误剥离（仅头部连续块才剥）", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const md = [
        "---",
        "ai_summary: only",
        "---",
        "正文开始",
        "",
        "---",
        "",
        "水平线后的正文",
      ].join("\n");

      const result = parseFrontmatter(md);
      expect(result.frontmatter?.aiSummary).toBe("only");
      // body 内部的 --- 应保留
      expect(result.body).toContain("水平线后的正文");
      expect(result.body).toContain("---");
      expect(warn).not.toHaveBeenCalled();
    });

    it("空首块 + 残留块 → null frontmatter 但 body 仍被清理", () => {
      const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
      const md = ["---", "---", "---", "ai_summary: 残留", "---", "正文"].join("\n");

      const result = parseFrontmatter(md);
      // 首块为空 → frontmatter null
      expect(result.frontmatter).toBeNull();
      // 残留块被剥离
      expect(result.body).toBe("正文");
      expect(result.body).not.toContain("---");
      expect(warn).toHaveBeenCalled();
    });
  });
});
