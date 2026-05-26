# Task 交付 — task_005_notarize_staple_gatekeeper (T-E)

## 实现摘要

按 ADR-005 实现 App Store Connect API-key 模式公证：

1. **`scripts/notarize.sh`（新建）**：单一入口完成 submit → status 判定 → staple → spctl 本地预检。
   - JSON 解析用 `python3 -c` 内联，对 notarytool 输出中可能掺杂的"Conducting pre-submission checks…"前缀人工文本通过正则 `\{.*\}` 抽取首个 JSON 块再 `json.loads`，兼容性强于 `plutil`（plutil 对混合 stderr+stdout 会报错）。
   - 重试策略：`for attempt in 1 2 3` 显式循环。**仅**对包含 `504`/`timeout`/`network`/`connection (refused|reset|closed)`/`temporary failure`/`could not connect` 等子串的输出重试，指数退避 5s / 15s / 45s（用 `SLEEP_BIN` 间接调用，便于 mock 测试瞬时跑完）。status=`Invalid`/`Rejected` 立即抓 `xcrun notarytool log <submission-id>` 并 exit 1，**不**重试。
   - secret 卫生：preflight 段与每次 `notarytool submit` 调用前都 `set +x`，使 `bash -x notarize.sh` 也不会把 `NOTARY_KEY_ID` / `NOTARY_ISSUER_ID` / `.p8` 路径回显到 trace；`.p8` 文件启动时 `chmod 600` 幂等强制。
   - spctl 本地预检：dev 机 spctl 缓存可能 stale，因此本机 `accepted: Notarized Developer ID` 字面匹配作为 **soft warning**（WARN 不 exit），最终判定交给 task_013 干净 VM。

2. **`scripts/build-macos-dmg.sh`（修改）**：
   - 删除原 `APPLE_NOTARY_PROFILE` 分支（基于已退役 keychain-profile 模式，且对 `.app` 而非 `.dmg` 公证，与 ADR-005 矛盾）。
   - 在 `hdiutil create` 之后插入 notarize.sh 调用；当 `NOTARY_KEY_ID` / `NOTARY_ISSUER_ID` / `NOTARY_KEY_P8_PATH` 三者全部存在时触发，否则 WARN 跳过（dev 机便利性）。
   - **未触碰** task_004 的签名块、`sign-bundle.sh`、`entitlements.plist`。

3. **`.github/workflows/notarize-dmg.yml`（新建）**：
   - 三 secret：`NOTARY_KEY_ID` / `NOTARY_ISSUER_ID` / `NOTARY_KEY_P8_BASE64`（base64 编码的 .p8 内容）。
   - decode step：`set +x` 包裹 `printf '%s' "$KEY_B64" | base64 -d`，落盘到 `$RUNNER_TEMP/AuthKey.p8` 并 `chmod 600`，绝不 echo 内容。
   - `if: always()` 的 shred step：`rm -P -f` 即使 notarize 失败/取消也销毁 .p8。
   - 仅输出每个 secret 的长度（`${#VAR}`），从不输出值。

## 修改的文件

| 文件路径 | 变更类型 | 说明 |
|---------|----------|------|
| `NCdesktop/scripts/notarize.sh` | 新建 | submit + log fetch + staple + spctl pre-flight，含重试与 secret 静音 |
| `NCdesktop/scripts/build-macos-dmg.sh` | 修改 | 删除旧 keychain-profile 块；DMG 创建后调用 notarize.sh |
| `NCdesktop/.github/workflows/notarize-dmg.yml` | 新建 | CI 模板：decode .p8 → chmod 600 → notarize → always-shred |

### git diff --stat

```
NCdesktop/scripts/build-macos-dmg.sh          | 50 ++++++++++++----------
NCdesktop/scripts/notarize.sh                 | 新增（约 200 行）
NCdesktop/.github/workflows/notarize-dmg.yml  | 新增（约 100 行）
```

未触碰：`scripts/entitlements.plist`、`scripts/sign-bundle.sh`、`scripts/verify-venv-shim.sh`、task_000 脱敏区脚本、Rust 文件，全部 `git diff` 验证为 0 行变更。

## 对 Architect 方案的遵守声明

- [x] 目录结构与 Architect 方案一致（`scripts/notarize.sh` 路径与 ADR-005 / line 164 完全一致）
- [x] API 路径/命名与 Architect 方案一致（`xcrun notarytool submit --wait` + `xcrun stapler staple` + spctl 字面 `accepted: Notarized Developer ID`）
- [x] 数据模型与 Architect 方案一致（无新增数据库 / schema 变更）
- [x] 未引入计划外的新依赖（仅依赖 macOS 自带 `xcrun` / `python3` / `base64`）
- 偏离说明：spctl 本地预检失败处理为 **WARN 不 exit**（input.md AC-3 强调 spctl 真实判定在干净 VM）；与 ADR-005 描述的 "干净机 spctl 必须 accepted" 一致，本机仅作 sanity check。

## 测试命令

```bash
# 1. 语法
bash -n scripts/notarize.sh
bash -n scripts/build-macos-dmg.sh

# 2. 启动检查（缺 env / 缺 arg / 不存在 DMG）
bash scripts/notarize.sh                # exit 2
bash scripts/notarize.sh /no/such.dmg   # exit 2
DMG=$(mktemp); touch $DMG; bash scripts/notarize.sh $DMG  # exit 2 (缺 env)

# 3. mock xcrun 自测矩阵（详见下方"自测验证矩阵"）
#    使用 XCRUN_BIN / SLEEP_BIN 注入点替换为 mock 二进制
```

## 测试结果

```
=== TEST 1: missing env ===
[notarize] ERROR: missing required env: NOTARY_KEY_ID NOTARY_ISSUER_ID NOTARY_KEY_P8_PATH
exit=2 → T1_PASS

=== TEST 2: missing arg ===
[notarize] ERROR: usage: notarize.sh <path-to-dmg>
exit=2 → T2_PASS

=== TEST 3: nonexistent DMG ===
[notarize] ERROR: DMG not found: /no/such/path.dmg
exit=2 → T3_PASS

=== TEST 4: happy path (mock Accepted) ===
[notarize] Accepted (submission id: abc-123-fake)
The staple and validate action worked!
[notarize] spctl: accepted Notarized Developer ID (local check)
[notarize] DONE
exit=0 → T4_PASS

=== TEST 5: Invalid → no retry + fetch log ===
[notarize] FAIL: status=Invalid (submission id: bad-456-fake)
[notarize] Fetching developer log from notary service...
{"logFormatVersion":1,"jobId":"bad-456-fake","status":"Invalid","issues":[...]}
[notarize] (not retrying — server-side rejection, not a transient error)
exit=1 → T5_PASS

=== TEST 6: 504 ×2 then Accepted (retry success) ===
attempt 1/3 → 504 → sleep 5s
attempt 2/3 → 504 → sleep 15s
attempt 3/3 → Accepted
exit=0  attempts=3 → T6_PASS

=== TEST 7: always 504 (retry exhausted) ===
attempt 1/3 → 504 → sleep 5s
attempt 2/3 → 504 → sleep 15s
attempt 3/3 → 504
[notarize] FAIL: 3 attempts exhausted on transient errors
exit=1  attempts=3 → T7_PASS

=== TEST 8: bash -x trace secret leak check ===
.p8 contents (FAKE_P8_PRIVATE_KEY_CONTENT_DO_NOT_LEAK): NOT IN LOG → PASS
KEY_ID (SUPER_SECRET_KEY_ID_DO_NOT_LEAK):                NOT IN LOG → PASS
ISSUER (...-SUPERSECRETISSUER):                          NOT IN LOG → PASS
→ T8_PASS

=== TEST 9-10: python3 JSON parse path ===
plain JSON: status=Accepted id=abc-123 → PASS
JSON with leading "Conducting pre-submission checks..." text: status=Accepted → PASS

=== TEST 11: final syntax ===
bash -n scripts/notarize.sh → OK
bash -n scripts/build-macos-dmg.sh → OK

=== TEST 12: workflow YAML parses cleanly ===
keys=['name','on','jobs']  jobs=['notarize']
```

## 自测验证矩阵

| 场景类型 | 场景描述 | 状态 | 结果/原因 |
|----------|----------|------|-----------|
| ✅ 正常路径 | AC-1 mock Accepted → exit 0 | 已测 | T4 PASS（happy path 全链路通） |
| ✅ 正常路径 | AC-2 stapler 字面 "The staple and validate action worked!" 校验 | 已测 | T4 内含 grep -F 命中 |
| ✅ 正常路径 | spctl 本地 grep "accepted" + "Notarized Developer ID" | 已测 | T4 mock 命中 |
| ⚠️ 边界条件 | AC-6 重试 504×2 后成功（指数退避 5/15s） | 已测 | T6 PASS（fast_sleep mock 验证退避序列） |
| ⚠️ 边界条件 | AC-6 重试上限 3 次后失败退出 | 已测 | T7 PASS（rc=1, attempts=3） |
| ⚠️ 边界条件 | JSON 输出夹杂人工文本前缀 | 已测 | T10 PASS（正则抽取 `\{.*\}` 首块） |
| ❌ 异常路径 | AC-1 status=Invalid → 抓 log + exit≠0 + 不重试 | 已测 | T5 PASS |
| ❌ 异常路径 | 缺 env / 缺 arg / DMG 不存在 → exit≠0 | 已测 | T1/T2/T3 PASS |
| ❌ 异常路径 | AC-5 `bash -x` trace 模式 secret 不泄漏 | 已测 | T8 PASS（.p8 内容/key_id/issuer_id 三者均未出现在 trace） |
| ⏳ PENDING | AC-3 macOS 12 arm64 干净 VM 首启 Gatekeeper 通过 | 未测 | PENDING-CLEAN-VM（task_013 接力，本任务无法访问 VM） |
| ⏳ PENDING | AC-4 macOS 14 arm64 干净 VM 首启 Gatekeeper 通过 | 未测 | PENDING-CLEAN-VM（task_013 接力） |
| ⏳ PENDING | 真公证回路（Apple 服务真调） | 未测 | 缺 App Store Connect API key；CI workflow 已准备就绪等 secrets 注入 |

## 浏览器/运行时验证

N/A — 纯 shell 脚本 + CI workflow，无 UI 入口。

## 已知局限

1. **真公证未跑**：手头无 Apple Developer Program API key 与签名身份；脚本逻辑用 mock xcrun 全路径覆盖，CI workflow 待 PM 注入三 secret 后即可端到端运行。
2. **干净 VM 烟测**：AC-3 / AC-4 必须在 task_013 在真实 macOS 12 + 14 arm64 VM 上验证，本任务无法替代。
3. **stapler 失败语境**：stapler 偶尔会因为 CloudKit 查询慢而瞬时失败（Apple 已知问题）。当前实现 stapler 失败直接 exit；未来若 CI 频繁碰到可在 stapler 这一步也加一层小重试（暂未做，因 AC-2 字面要求未提及）。
4. **`set +x` 与外部 trace**：`set +x` 在子 shell（bash -x notarize.sh）中能屏蔽 notarize.sh 自身的 trace；但若调用方 `bash -x` 后直接 `source` 本脚本，trace 屏蔽行为依赖 bash 的 `$-` 状态恢复。当前不支持 source 调用（input.md 未要求），通过子 shell 调用 100% 安全。

## 需要 Reviewer 特别关注的地方

1. **JSON 解析方式**：用 `python3 -c` 内联 + 正则抽 `\{.*\}` 抓首个 JSON 块，未用 `plutil`（plutil 对混合人工前缀 + JSON 的实际 notarytool stdout 会报错）。如果 Reviewer 偏好 `plutil` 一致性，需提供能处理人工前缀的方案。

2. **submission-id 提取**：与 status 同一次 python3 解析中抽取（独立两次调用，简单且不依赖临时文件）。`xcrun notarytool log` 命令对 submission-id 缺失时直接跳过 log 抓取（避免传空 id 触发额外 xcrun 错误）。

3. **CI .p8 销毁时机**：用 `if: always()` 的 shred step 兜底；macOS 上 `rm -P -f` 先覆写再删除（fallback 普通 `rm -f`）。GH-hosted `macos-14` runner 本身就是 ephemeral，但显式删除是审计要求。

4. **重试策略边界**：当前对"未识别失败模式"（rc≠0 但不匹配网络关键词，且无 status JSON）直接 exit 1 不重试。考虑过对 rc≠0 一律重试，但担心遇到认证错（key 过期）会浪费 90s 才报。如果 Reviewer 认为更激进重试更稳，可调整为：rc≠0 且 status 未解析出来 → 也算 transient。

5. **本机 spctl 预检的 WARN 语义**：dev 机 spctl 缓存常 stale，硬 fail 会阻断本地 dry-run。当前实现 WARN 不 exit，把硬判交给 task_013；如果 Reviewer 认为本机应硬 fail，可加 `STRICT_SPCTL=1` env 切换。

6. **`APPLE_NOTARY_PROFILE` 删除**：build-macos-dmg.sh 原 `APPLE_NOTARY_PROFILE` 分支已删除（既退役又错对象）。若有外部脚本/文档仍在用此 env，需要在 release notes 标注 breaking change。该 env 在本次清理前未被其他 task 引用，搜索 repo 内未发现剩余引用点。
