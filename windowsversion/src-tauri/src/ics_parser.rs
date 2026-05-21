//! iCalendar (.ics) 解析器
//!
//! 职责：
//! - 解析 VEVENT（SUMMARY/DTSTART/DTEND/RRULE/LOCATION/DESCRIPTION/UID）
//! - 从 SUMMARY 用正则提取 course_code（如 ECON 101）
//! - 从 DESCRIPTION 提取 instructor
//! - 将 RRULE 展开为指定时间窗口内的独立实例
//! - 基于 UID + DTSTART 去重

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc, Weekday};
use std::collections::HashSet;

/// 解析后的课程事件（尚未持久化，供用户预览选择）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedEvent {
    /// 临时 ID（FNV hash of uid+dtstart），用于去重和前端选择
    pub temp_id: String,
    pub title: String,
    pub course_code: Option<String>,
    pub instructor: Option<String>,
    pub location: Option<String>,
    pub start_time: String,  // RFC3339 UTC
    pub end_time: String,    // RFC3339 UTC
    pub recurrence_rule: Option<String>,
    pub day_of_week: Vec<i64>,
    pub description: Option<String>,
    pub source_uid: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// 公共 API
// ─────────────────────────────────────────────────────────────────────────────

/// 解析 .ics 内容，时间窗口 = 当前起 4 个月（120 天）
pub fn parse_ics(content: &str) -> Result<Vec<ParsedEvent>, String> {
    let now = Utc::now();
    let horizon = now + Duration::days(120);
    parse_ics_with_window(content, now, horizon)
}

/// 解析 .ics 内容，使用自定义时间窗口（方便测试注入）
pub fn parse_ics_with_window(
    content: &str,
    from: DateTime<Utc>,
    horizon: DateTime<Utc>,
) -> Result<Vec<ParsedEvent>, String> {
    let reader = ical::IcalParser::new(content.as_bytes());
    let mut events: Vec<ParsedEvent> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for calendar in reader {
        let cal = calendar.map_err(|e| format!("解析日历失败: {e}"))?;
        for vevent in cal.events {
            let props = &vevent.properties;

            let get = |key: &str| -> Option<String> {
                props
                    .iter()
                    .find(|p| p.name.eq_ignore_ascii_case(key))
                    .and_then(|p| p.value.clone())
            };

            let summary = get("SUMMARY").unwrap_or_default();
            let uid = get("UID");
            let location = get("LOCATION");
            let description = get("DESCRIPTION");
            let rrule = get("RRULE");

            // 起止时间
            let dtstart_raw = match get("DTSTART") {
                Some(v) => v,
                None => continue,
            };
            let dtend_raw = get("DTEND").unwrap_or_else(|| {
                parse_dt(&dtstart_raw)
                    .map(|dt| fmt_rfc3339(dt + Duration::hours(1)))
                    .unwrap_or_else(|| dtstart_raw.clone())
            });

            let start_dt = match parse_dt(&dtstart_raw) {
                Some(dt) => dt,
                None => continue,
            };
            let end_dt = parse_dt(&dtend_raw).unwrap_or(start_dt + Duration::hours(1));
            let duration = end_dt - start_dt;

            let course_code = extract_course_code(&summary);
            let instructor = description.as_deref().and_then(extract_instructor);

            if let Some(ref rule) = rrule {
                // 展开重复事件
                for (inst_start, inst_end) in
                    expand_rrule(rule, start_dt, duration, from, horizon)
                {
                    let key = dedup_key(uid.as_deref().unwrap_or(&summary), inst_start);
                    if !seen.insert(key.clone()) {
                        continue;
                    }
                    events.push(ParsedEvent {
                        temp_id: fnv_id(&key),
                        title: summary.clone(),
                        course_code: course_code.clone(),
                        instructor: instructor.clone(),
                        location: location.clone(),
                        start_time: fmt_rfc3339(inst_start),
                        end_time: fmt_rfc3339(inst_end),
                        recurrence_rule: Some(rule.clone()),
                        day_of_week: vec![weekday_num(inst_start.weekday())],
                        description: description.clone(),
                        source_uid: uid.clone(),
                    });
                }
            } else {
                // 非重复事件
                if start_dt < from || start_dt > horizon {
                    continue;
                }
                let key = dedup_key(uid.as_deref().unwrap_or(&summary), start_dt);
                if !seen.insert(key.clone()) {
                    continue;
                }
                events.push(ParsedEvent {
                    temp_id: fnv_id(&key),
                    title: summary,
                    course_code,
                    instructor,
                    location,
                    start_time: fmt_rfc3339(start_dt),
                    end_time: fmt_rfc3339(end_dt),
                    recurrence_rule: None,
                    day_of_week: vec![weekday_num(start_dt.weekday())],
                    description,
                    source_uid: uid,
                });
            }
        }
    }

    events.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    Ok(events)
}

// ─────────────────────────────────────────────────────────────────────────────
// 提取工具
// ─────────────────────────────────────────────────────────────────────────────

/// 从课程名提取课程号，例 "ECON 101 - Intro" → "ECON 101"
pub fn extract_course_code(summary: &str) -> Option<String> {
    regex_course().find(summary).map(|m| m.as_str().to_string())
}

/// 从描述中提取教授名
pub fn extract_instructor(desc: &str) -> Option<String> {
    for line in desc.lines() {
        let line = line.trim();
        if let Some(rest) = line
            .strip_prefix("Instructor:")
            .or_else(|| line.strip_prefix("Instructor :"))
        {
            let name = rest.trim().to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }
        if let Some(rest) = line
            .strip_prefix("Prof.")
            .or_else(|| line.strip_prefix("Professor "))
            .or_else(|| line.strip_prefix("Prof "))
        {
            let name = rest
                .trim()
                .split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ");
            if !name.is_empty() {
                return Some(format!("Prof. {name}"));
            }
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// RRULE 展开
// ─────────────────────────────────────────────────────────────────────────────

/// 展开 RRULE，返回 (start, end) 时间对列表
/// 覆盖大学课表的 WEEKLY / DAILY 频率；其它频率不展开
fn expand_rrule(
    rrule: &str,
    base_start: DateTime<Utc>,
    duration: Duration,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Vec<(DateTime<Utc>, DateTime<Utc>)> {
    let mut result = Vec::new();
    let freq = rrule_val(rrule, "FREQ").unwrap_or_default();

    if freq.eq_ignore_ascii_case("WEEKLY") {
        let interval: i64 = rrule_val(rrule, "INTERVAL")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);

        let byday = rrule_val(rrule, "BYDAY");
        let target_days: Vec<i64> = if let Some(days_str) = byday {
            days_str
                .split(',')
                .filter_map(|d| ical_day_num(d.trim()))
                .collect()
        } else {
            vec![weekday_num(base_start.weekday())]
        };

        let until_rrule = rrule_val(rrule, "UNTIL")
            .and_then(|v| parse_dt(&v))
            .unwrap_or(until);
        let effective_until = until_rrule.min(until);

        let count_limit: usize = rrule_val(rrule, "COUNT")
            .and_then(|v| v.parse().ok())
            .unwrap_or(usize::MAX);

        // 从 base_start 起找第一个不早于 from 的完整周
        let mut week_anchor = base_start;
        if week_anchor < from {
            let weeks_behind = ((from - week_anchor).num_days() / 7).max(0);
            week_anchor = week_anchor + Duration::weeks(weeks_behind * interval);
        }

        let mut count = 0usize;
        loop {
            if week_anchor > effective_until {
                break;
            }
            for &dow in &target_days {
                let offset = (dow - weekday_num(week_anchor.weekday())).rem_euclid(7);
                let inst_start = week_anchor + Duration::days(offset);
                if inst_start < from || inst_start > effective_until {
                    continue;
                }
                result.push((inst_start, inst_start + duration));
                count += 1;
                if count >= count_limit {
                    return result;
                }
            }
            week_anchor = week_anchor + Duration::weeks(interval);
        }
    } else if freq.eq_ignore_ascii_case("DAILY") {
        let interval: i64 = rrule_val(rrule, "INTERVAL")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1);
        let until_rrule = rrule_val(rrule, "UNTIL")
            .and_then(|v| parse_dt(&v))
            .unwrap_or(until);
        let effective_until = until_rrule.min(until);
        let count_limit: usize = rrule_val(rrule, "COUNT")
            .and_then(|v| v.parse().ok())
            .unwrap_or(usize::MAX);

        let mut cur = base_start;
        if cur < from {
            let steps = ((from - cur).num_days() + interval - 1) / interval;
            cur = cur + Duration::days(steps * interval);
        }
        let mut count = 0;
        while cur <= effective_until && count < count_limit {
            result.push((cur, cur + duration));
            cur = cur + Duration::days(interval);
            count += 1;
        }
    }

    result
}

// ─────────────────────────────────────────────────────────────────────────────
// 小工具
// ─────────────────────────────────────────────────────────────────────────────

/// 解析 iCal 日期时间字符串 → UTC
pub fn parse_dt(s: &str) -> Option<DateTime<Utc>> {
    let s = s.trim();

    // 20260407T090000Z
    if s.ends_with('Z') && s.len() >= 15 {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&s[..15], "%Y%m%dT%H%M%S") {
            return Some(Utc.from_utc_datetime(&dt));
        }
    }

    // 20260407T090000 (floating / TZID)
    if s.len() >= 15 && s.contains('T') {
        let clean: String = s.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if clean.len() >= 15 {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&clean[..15], "%Y%m%dT%H%M%S") {
                return Some(Utc.from_utc_datetime(&dt));
            }
        }
    }

    // 20260407
    if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(d) = NaiveDate::parse_from_str(s, "%Y%m%d") {
            let dt = d.and_hms_opt(0, 0, 0)?;
            return Some(Utc.from_utc_datetime(&dt));
        }
    }

    None
}

fn fmt_rfc3339(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

fn weekday_num(wd: Weekday) -> i64 {
    match wd {
        Weekday::Mon => 1,
        Weekday::Tue => 2,
        Weekday::Wed => 3,
        Weekday::Thu => 4,
        Weekday::Fri => 5,
        Weekday::Sat => 6,
        Weekday::Sun => 7,
    }
}

fn ical_day_num(s: &str) -> Option<i64> {
    let alpha: String = s.chars().filter(|c| c.is_alphabetic()).collect();
    match alpha.to_uppercase().as_str() {
        "MO" => Some(1),
        "TU" => Some(2),
        "WE" => Some(3),
        "TH" => Some(4),
        "FR" => Some(5),
        "SA" => Some(6),
        "SU" => Some(7),
        _ => None,
    }
}

fn rrule_val(rrule: &str, key: &str) -> Option<String> {
    for part in rrule.split(';') {
        if let Some((k, v)) = part.split_once('=') {
            if k.trim().eq_ignore_ascii_case(key) {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn dedup_key(uid: &str, dt: DateTime<Utc>) -> String {
    format!("{uid}:{}", dt.to_rfc3339())
}

/// FNV-1a 64-bit hash，生成 temp_id
fn fnv_id(input: &str) -> String {
    let mut h: u64 = 14695981039346656037;
    for b in input.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    format!("{h:016x}")
}

/// 懒加载课程号正则
fn regex_course() -> &'static regex_lite::Regex {
    static RE: std::sync::OnceLock<regex_lite::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex_lite::Regex::new(r"[A-Z]{2,5}\s*\d{3,4}").expect("regex"))
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── 测试辅助 ───────────────────────────────────────────────────────────────

    fn mk_ics(uid: &str, summary: &str, dtstart: &str, dtend: &str) -> String {
        format!(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:{uid}\r\nSUMMARY:{summary}\r\nDTSTART:{dtstart}\r\nDTEND:{dtend}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
        )
    }

    fn mk_rrule_ics(uid: &str, summary: &str, dtstart: &str, dtend: &str, rrule: &str) -> String {
        format!(
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:{uid}\r\nSUMMARY:{summary}\r\nDTSTART:{dtstart}\r\nDTEND:{dtend}\r\nRRULE:{rrule}\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n"
        )
    }

    /// 覆盖 2099 年全年的测试窗口
    fn win2099() -> (DateTime<Utc>, DateTime<Utc>) {
        (
            Utc.with_ymd_and_hms(2099, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2099, 12, 31, 23, 59, 59).unwrap(),
        )
    }

    /// 便捷解析：注入 2099 窗口
    fn parse(ics: &str) -> Vec<ParsedEvent> {
        let (from, to) = win2099();
        parse_ics_with_window(ics, from, to).expect("解析失败")
    }

    // ── 基础解析 ───────────────────────────────────────────────────────────────

    #[test]
    fn parse_single_event_in_window() {
        let ics = mk_ics(
            "uid-001",
            "ECON 101 - Intro to Microeconomics",
            "20991001T090000Z",
            "20991001T101500Z",
        );
        let events = parse(&ics);
        assert_eq!(events.len(), 1);
        let e = &events[0];
        assert_eq!(e.title, "ECON 101 - Intro to Microeconomics");
        assert_eq!(e.course_code.as_deref(), Some("ECON 101"));
        assert!(e.start_time.contains("2099"));
    }

    #[test]
    fn parse_event_outside_window_is_skipped() {
        let (from, to) = win2099();
        let ics = mk_ics("uid-002", "OLD 100", "20200101T090000Z", "20200101T101500Z");
        let events = parse_ics_with_window(&ics, from, to).unwrap();
        assert_eq!(events.len(), 0, "窗口外事件应被过滤");
    }

    // ── course_code 提取 ───────────────────────────────────────────────────────

    #[test]
    fn extract_course_code_variants() {
        assert_eq!(
            extract_course_code("ECON 101 - Intro"),
            Some("ECON 101".to_string())
        );
        assert_eq!(
            extract_course_code("CS231N Deep Learning"),
            Some("CS231".to_string())
        );
        assert_eq!(
            extract_course_code("MATH 3010 Calc"),
            Some("MATH 3010".to_string())
        );
        assert_eq!(extract_course_code("Office Hours"), None);
    }

    // ── instructor 提取 ────────────────────────────────────────────────────────

    #[test]
    fn extract_instructor_formats() {
        assert_eq!(
            extract_instructor("Instructor: John Smith\nRoom: 302"),
            Some("John Smith".to_string())
        );
        assert_eq!(
            extract_instructor("Prof. Jane Doe\nECON 101"),
            Some("Prof. Jane Doe".to_string())
        );
        assert_eq!(
            extract_instructor("Professor Alan Turing"),
            Some("Prof. Alan Turing".to_string())
        );
        assert_eq!(extract_instructor("No instructor info here"), None);
    }

    // ── RRULE 展开 — 每周一 ────────────────────────────────────────────────────

    #[test]
    fn rrule_weekly_expands_correctly() {
        let ics = mk_rrule_ics(
            "uid-rrule-1",
            "CS 231 - Algorithms",
            "20991006T090000Z", // 2099-10-06 周一
            "20991006T101500Z",
            "FREQ=WEEKLY;COUNT=6;BYDAY=MO",
        );
        let events = parse(&ics);
        assert_eq!(events.len(), 6, "应展开 6 个实例，实际: {}", events.len());
        for e in &events {
            assert_eq!(e.course_code.as_deref(), Some("CS 231"));
            assert_eq!(e.day_of_week, vec![1]); // 1=Mon
        }
    }

    // ── RRULE 展开 — 每周一三五 ────────────────────────────────────────────────

    #[test]
    fn rrule_mwf_pattern() {
        let ics = mk_rrule_ics(
            "uid-rrule-mwf",
            "PHIL 220",
            "20991006T110000Z",
            "20991006T115000Z",
            "FREQ=WEEKLY;COUNT=9;BYDAY=MO,WE,FR",
        );
        let events = parse(&ics);
        assert_eq!(events.len(), 9, "MWF 应展开 9 个实例，实际: {}", events.len());
        let dows: Vec<i64> = events.iter().map(|e| e.day_of_week[0]).collect();
        assert!(dows.contains(&1), "应有周一");
        assert!(dows.contains(&3), "应有周三");
        assert!(dows.contains(&5), "应有周五");
    }

    // ── 去重 ───────────────────────────────────────────────────────────────────

    #[test]
    fn duplicate_uid_dtstart_deduplicated() {
        let event = mk_ics("uid-dup", "HIST 150", "20991010T130000Z", "20991010T141500Z");
        let double = format!("{event}{event}");
        let events = parse(&double);
        assert_eq!(events.len(), 1, "重复事件应去重，只保留一条");
    }

    // ── temp_id 唯一性 ─────────────────────────────────────────────────────────

    #[test]
    fn temp_ids_are_unique_across_events() {
        let ics = mk_rrule_ics(
            "uid-unique",
            "MATH 301",
            "20991006T140000Z",
            "20991006T151500Z",
            "FREQ=WEEKLY;COUNT=4;BYDAY=MO",
        );
        let events = parse(&ics);
        let ids: HashSet<&str> = events.iter().map(|e| e.temp_id.as_str()).collect();
        assert_eq!(ids.len(), events.len(), "每个实例的 temp_id 应唯一");
    }

    // ── parse_dt 边界 ──────────────────────────────────────────────────────────

    #[test]
    fn parse_dt_formats() {
        assert!(parse_dt("20260407T090000Z").is_some(), "UTC Z 格式");
        assert!(parse_dt("20260407").is_some(), "纯日期格式");
        assert!(parse_dt("20260407T090000").is_some(), "浮动时间");
        assert!(parse_dt("invalid").is_none(), "无效格式");
    }
}
