# EasyDict 翻译结果复刻 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让 Translator 应用在输入英文单词时像 EasyDict 一样展示多结果（多词性释义、网络释义、音标、词形、同反义词/搭配/词源、DeepL 备选），覆盖现有 5 个服务。

**Architecture:** 在现有 `TranslateResult`/`DictionaryResult` 模型上扩展字段（alternatives + synonyms/antonyms/collocation/etymology），把 `part_abbreviation` 提为 core 公共函数；前端 `ResultBody` 改为渲染 `source_dictionary`、`DictionaryDetails` 按 EasyDict 顺序重排并补全区块；后端 Bing/Google 补字典解析路径、DeepL 捕获 alternatives。Youdao 后端已完成，Phase 1 前端即可见效。

**Tech Stack:** Rust (edition 2024, rust 1.96, reqwest/serde/md5/aes), Tauri 2, React 18 + TS 5.6 + Vite 5 + Tailwind 3, @fluent/bundle i18n。后端测试 wiremock；前端无 JS 测试框架，靠 `typecheck`/`build`/`locales:check` + 运行时核对。

## Global Constraints

- 工作分支 `feat/easydict-result-replication`，不直接提交 main；每 Task/Phase 频繁提交。
- Rust 字典字段统一 `#[serde(default, skip_serializing_if = "Vec::is_empty")]`，`Option` 字段 `skip_serializing_if = "Option::is_none"`。
- `vendor/easydict/` 为只读参考，不得修改。
- 12 个 `.ftl` locale 文件须保持同步，`npm run locales:check` 必须通过。
- `bindings.ts` 与 `model.rs` 手工保持同步（无 ts-rs）。
- 后端新增/改动解析均走 TDD：先用 EasyDict 样例 JSON 写 wiremock/单元测试，再实现至通过。
- 前端无测试框架：以 `npm run typecheck`、`npm run build`、`npm run locales:check` 通过为编译门，运行时行为靠启动应用人工核对。

## File Structure

**Rust (`crates/core/src/`)**
- `model.rs` — `TranslateResult.alternatives`、`DictionaryResult.{synonyms,antonyms,collocation,etymology}`、`is_empty()`、`pub fn part_abbreviation`（从 youdao.rs 提升并扩展映射表）。
- `services/youdao.rs` — 改用 `crate::model::part_abbreviation`，移除本地副本（Phase 2 核对）。
- `services/bing.rs` — 新增 v7 dict 请求 + `tlookupv3` lookup + 解析填充 source_dictionary（Phase 3）。
- `services/google.rs` — 新增 webApp 路径 + `tk` 签名 + dict 块解析（Phase 4）。
- `services/deepl.rs` — 捕获 `alternatives`（Phase 5）。
- 各 service 的 `TranslateResult { ... }` 字面量构造补 `alternatives: Vec::new()`。

**前端 (`ui/src/`)**
- `types/bindings.ts` — 同步新字段。
- `App.tsx` — `ResultBody`/`DictionaryDetails`/`ResultCard` 重写（source 字典、EasyDict 顺序、大词头、alternatives、底部工具栏）；把 `sourceText` 透传到卡片。
- `locales/*.ftl` ×12 — 新增 `synonyms/antonyms/collocation/etymology/alternatives`。

**参考（只读）**
- `vendor/easydict/Easydict/Swift/Service/Model/QueryResult.swift`（`partAbbreviation` 全表）、`WordResultView/EZWordResultView.m`（渲染顺序）、各 service `+Translate.swift`、`google-translate-sign.js`、`Youdao/Model/DictJSONExample/v4/*.json`。

---

## Phase 1 — 模型 + 前端 + i18n（Youdao 多结果立即可见）

### Task 1: 扩展 Rust 数据模型

**Files:**
- Modify: `crates/core/src/model.rs`（`TranslateResult`、`DictionaryResult`、`is_empty`、新增 `part_abbreviation` + tests）
- Modify: 所有构造 `TranslateResult { ... }` 字面量的文件（`grep -rn "TranslateResult {" crates/`）：`services/youdao.rs`、`services/bing.rs`、`services/google.rs`、`services/deepl.rs`、`services/openai.rs`，以及 `translator.rs`/`commands.rs` 中若有。

**Interfaces:**
- Produces: `TranslateResult.alternatives: Vec<String>`、`DictionaryResult.{synonyms,antonyms,collocation: Vec<DictionaryPart>, etymology: Option<String>}`、`pub fn part_abbreviation(part: &str) -> String`。

- [ ] **Step 1: 写失败测试**（在 `model.rs` 末尾追加 `#[cfg(test)] mod tests`，若已存在则并入）

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_result_is_empty_counts_new_fields() {
        assert!(DictionaryResult::default().is_empty());
        let mut d = DictionaryResult::default();
        d.synonyms.push(DictionaryPart { part: None, means: vec!["fast".into()] });
        assert!(!d.is_empty(), "synonyms should count");
        let mut d = DictionaryResult::default();
        d.etymology = Some("from Old English".into());
        assert!(!d.is_empty(), "etymology should count");
    }

    #[test]
    fn translate_result_skips_empty_alternatives_in_json() {
        let base = TranslateResult {
            service_id: ServiceId::Youdao,
            service_name: "Youdao".into(),
            from: None,
            to: "zh-Hans".into(),
            text: "好".into(),
            audio_url: None,
            detected_source: None,
            elapsed_ms: 0,
            dictionary: None,
            source_dictionary: None,
            target_dictionary: None,
            extra: None,
            alternatives: Vec::new(),
        };
        let json = serde_json::to_string(&base).unwrap();
        assert!(!json.contains("alternatives"));
        let mut with_alts = base.clone();
        with_alts.alternatives = vec!["good".into(), "fine".into()];
        let json2 = serde_json::to_string(&with_alts).unwrap();
        assert!(json2.contains("\"alternatives\""));
    }

    #[test]
    fn part_abbreviation_maps_full_easydict_table() {
        assert_eq!(part_abbreviation("noun"), "n.");
        assert_eq!(part_abbreviation("形容词"), "adj.");
        assert_eq!(part_abbreviation("adj."), "adj."); // already-abbreviated passes through
        assert_eq!(part_abbreviation("linking verb"), "linkv.");
        assert_eq!(part_abbreviation("auxv"), "auxv.");
        assert_eq!(part_abbreviation("modal verb"), "modalv.");
        assert_eq!(part_abbreviation("determiner"), "det.");
        assert_eq!(part_abbreviation("abbreviation"), "abbr.");
        assert_eq!(part_abbreviation("infinitive"), "inf.");
        assert_eq!(part_abbreviation("participle"), "part.");
        assert_eq!(part_abbreviation("Web"), "Web");
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cargo test -p translator-core --lib model::tests`
Expected: FAIL（`alternatives` 字段不存在 / `part_abbreviation` 未定义或映射不全）

- [ ] **Step 3: 实现** — 在 `TranslateResult` 中 `text` 之后加：

```rust
    /// 备选译文（如 DeepL 的 alternatives）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<String>,
```

在 `DictionaryResult` 中 `tags` 之后加：

```rust
    /// 同义词（按词性分组）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub synonyms: Vec<DictionaryPart>,
    /// 反义词（按词性分组）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub antonyms: Vec<DictionaryPart>,
    /// 搭配（按词性分组）。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub collocation: Vec<DictionaryPart>,
    /// 词源。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etymology: Option<String>,
```

更新 `is_empty()`：

```rust
    pub fn is_empty(&self) -> bool {
        self.phonetics.is_empty()
            && self.parts.is_empty()
            && self.exchanges.is_empty()
            && self.simple_words.is_empty()
            && self.tags.is_empty()
            && self.synonyms.is_empty()
            && self.antonyms.is_empty()
            && self.collocation.is_empty()
            && self.etymology.is_none()
    }
```

在 `model.rs` 中（`DictionaryResult` impl 之前或文件末尾函数区）新增公共函数（从 youdao.rs 复制并扩展映射表，对齐 EasyDict `partAbbreviation`）：

```rust
/// 把词性全称/中文/已缩写形式归一为 EasyDict 风格缩写（n./v./adj./...）。
pub fn part_abbreviation(part: &str) -> String {
    let normalized = part.trim().trim_end_matches('.').to_ascii_lowercase();
    let mapped = match normalized.as_str() {
        "adjective" | "形容词" | "adj" => "adj.",
        "adverb" | "副词" | "adv" => "adv.",
        "verb" | "动词" | "v" => "v.",
        "noun" | "名词" | "n" => "n.",
        "pronoun" | "代词" | "pron" => "pron.",
        "preposition" | "介词" | "prep" => "prep.",
        "conjunction" | "连词" | "conj" => "conj.",
        "interjection" | "感叹词" | "int" | "interj" => "int.",
        "article" | "冠词" | "art" => "art.",
        "numeral" | "数词" | "num" => "num.",
        "linking verb" | "linkverb" | "linkv" => "linkv.",
        "auxiliary verb" | "auxverb" | "auxv" => "auxv.",
        "modal verb" | "modalverb" | "modalv" => "modalv.",
        "determiner" | "det" => "det.",
        "abbreviation" | "abbr" => "abbr.",
        "infinitive" | "inf" => "inf.",
        "participle" | "part" => "part.",
        "web" => "Web",
        _ => part.trim(),
    };
    mapped.to_string()
}
```

- [ ] **Step 4: 更新所有 `TranslateResult { ... }` 字面量** — `grep -rn "TranslateResult {" crates/`，每处补 `alternatives: Vec::new(),`（放在 `extra` 字段附近）。`DictionaryResult` 构造多用 `::default()` + `.push()`，新字段自动默认，无需改。

- [ ] **Step 5: 运行测试确认通过**

Run: `cargo test -p translator-core --lib`
Expected: PASS，且 `cargo build -p translator-core` 通过。

- [ ] **Step 6: 提交**

```bash
git add crates/core/src/model.rs crates/core/src/services/*.rs
git commit -m "feat(core): add alternatives + dict sub-fields, promote part_abbreviation"
```

### Task 2: 同步 bindings.ts

**Files:**
- Modify: `ui/src/types/bindings.ts`

- [ ] **Step 1: 改 `TranslateResult`** — 在 `text: string;` 后加：

```ts
  alternatives?: string[];
```

- [ ] **Step 2: 改 `DictionaryResult`** — 在 `tags?: string[];` 后加：

```ts
  synonyms?: DictionaryPart[];
  antonyms?: DictionaryPart[];
  collocation?: DictionaryPart[];
  etymology?: string | null;
```

- [ ] **Step 3: 验证**

Run: `npm --prefix ui run typecheck`
Expected: PASS（无类型错误）

- [ ] **Step 4: 提交**

```bash
git add ui/src/types/bindings.ts
git commit -m "feat(ui): sync bindings for alternatives + dict sub-fields"
```

### Task 3: 前端 i18n 新增 key

**Files:**
- Modify: `ui/src/locales/{ar,de,en,es,fr,it,ja,ko,pt,ru,zh-Hans,zh-Hant}.ftl`

**Interfaces:** Produces keys: `main-synonyms`、`main-antonyms`、`main-collocation`、`main-etymology`、`main-alternatives`、`main-open-link`。

- [ ] **Step 1: 在 `en.ftl` 追加**（参考现有 `main-` 前缀风格）：

```ftl
main-synonyms = Synonyms
main-antonyms = Antonyms
main-collocation = Collocation
main-etymology = Etymology
main-alternatives = Alternatives
main-open-link = Open link
```

- [ ] **Step 2: 在其余 11 个 `.ftl` 追加对应翻译**（zh-Hans: 同义词/反义词/搭配/词源/备选译文；zh-Hant: 同義詞/反義詞/搭配/詞源/備選譯文；ja: 同義語/反義語/共起/語源/代替訳；ko: 동의어/반의어/연어/어원/대체 번역；等）。

- [ ] **Step 3: 验证**

Run: `npm --prefix ui run locales:check`
Expected: PASS（所有 locale key 对齐）

- [ ] **Step 4: 提交**

```bash
git add ui/src/locales/*.ftl
git commit -m "feat(ui): add dictionary section i18n keys"
```

### Task 4: 前端重写结果卡片渲染

**Files:**
- Modify: `ui/src/App.tsx`（`ResultCard`、`ResultBody`、`DictionaryDetails`；并把 `sourceText` 从 `App` 透传到 `ResultsPanel`→`ResultCard`→`ResultBody`）

**Interfaces:**
- Consumes: `TranslateResult.alternatives`、`DictionaryResult.{synonyms,antonyms,collocation,etymology}`、`useT()` 的 `main-*` key。
- Produces: 1:1 EasyDict 顺序的结果卡片：大词头 → 译文+备选 → 音标 → 标签 → 词性释义 → 词形 → 同义词 → 反义词 → 搭配 → 网络短语 → 词源 → 底部工具栏(音频/复制/链接)。

> 前端无单测；以 typecheck+build+运行时核对为门。执行前先通读 `App.tsx` 中 `App` 组件的输入状态、`ResultsPanel` 调用处、`sourceDictionary` 提取（约 L170-260）以确定 `sourceText` 透传路径。

- [ ] **Step 1: 透传 `sourceText`** — 在 `App` 中把当前输入文本作为 `sourceText` 传入 `<ResultsPanel sourceText={...} ... />`；`ResultsPanel`→`ResultCard`→`ResultBody` 逐层加 `sourceText: string` prop。

- [ ] **Step 2: 重写 `ResultBody`** — 字典源改为 `result.source_dictionary ?? result.dictionary ?? result.target_dictionary`；结构改为：

```tsx
function ResultBody({ result, sourceText, copied, onCopied }) {
  const t = useT();
  const dictionary = result.source_dictionary ?? result.dictionary ?? result.target_dictionary ?? null;
  const showBigWord = isShortWord(sourceText) && dictionary != null;
  return (
    <div className="space-y-3">
      {showBigWord && (
        <p className="text-xl font-semibold break-all">{sourceText}</p>
      )}
      <div className="space-y-1">
        <p className="min-w-0 whitespace-pre-wrap text-sm leading-6">{result.text}</p>
        {result.alternatives && result.alternatives.length > 0 && (
          <ul className="list-disc pl-4 text-sm text-fg-subtle">
            {result.alternatives.map((alt, i) => (
              <li key={i}>{alt}</li>
            ))}
          </ul>
        )}
      </div>
      <DictionaryDetails dictionary={dictionary} />
      <ResultToolbar result={result} copied={copied} onCopied={onCopied} />
    </div>
  );
}
```

`isShortWord`（对齐 EasyDict `EZEnglishWordMaxLength=20`，其他语言 ≤7 字符）：

```tsx
function isShortWord(text: string): boolean {
  const trimmed = text.trim();
  if (!trimmed || trimmed.includes("\n")) return false;
  const isAsciiWord = /^[\p{L}\p{N}]+([-'\s][\p{L}\p{N}]+)*$/u.test(trimmed) && /^[\x00-\x7F]+$/.test(trimmed);
  if (isAsciiWord) return [...trimmed].length <= 20 && !trimmed.includes(" ");
  return [...trimmed].length <= 7;
}
```

- [ ] **Step 3: 重写 `DictionaryDetails`** — 按 EasyDict 顺序：音标 → 标签 → 词性释义(parts) → 词形(exchanges) → 同义词 → 反义词 → 搭配 → 网络短语(simple_words) → 词源。在现有实现基础上：把 `exchanges` 移到 `simple_words` 之前，并追加 synonyms/antonyms/collocation/etymology 区块。新区块复用 parts 的「词性标签 + means」行式渲染：

```tsx
function DictionaryDetails({ dictionary, showPhonetics = true }) {
  // ...现有 phonetics/tags/parts/exchanges/simpleWords 提取...
  const synonyms = dictionary?.synonyms ?? [];
  const antonyms = dictionary?.antonyms ?? [];
  const collocation = dictionary?.collocation ?? [];
  const etymology = dictionary?.etymology ?? null;
  if (全部为空) return null;
  return (
    <div className="space-y-2 border-t border-border pt-2 text-xs">
      {/* phonetics（现有） */}
      {/* tags（现有） */}
      {/* parts（现有） */}
      {/* exchanges（现有，移到 simpleWords 前） */}
      {synonyms.length > 0 && <PartGroup title={t("main-synonyms", null, "Synonyms")} parts={synonyms} />}
      {antonyms.length > 0 && <PartGroup title={t("main-antonyms", null, "Antonyms")} parts={antonyms} />}
      {collocation.length > 0 && <PartGroup title={t("main-collocation", null, "Collocation")} parts={collocation} />}
      {/* simple_words（现有） */}
      {etymology && <div className="text-fg-subtle"><span className="font-medium">{t("main-etymology", null, "Etymology")}: </span>{etymology}</div>}
    </div>
  );
}

function PartGroup({ title, parts }) {
  return (
    <div className="space-y-1">
      <div className="font-medium text-fg-subtle">{title}</div>
      {parts.map((p, i) => (
        <div key={i} className="flex gap-2">
          {p.part && <span className="w-10 shrink-0 text-fg-subtle">{p.part}</span>}
          <span className="min-w-0 text-fg">{p.means.join("; ")}</span>
        </div>
      ))}
    </div>
  );
}
```

- [ ] **Step 4: 新增 `ResultToolbar`** — 把原内联 audio/copy 移到底部工具栏，并加「链接」按钮（打开服务词典页）。服务 wordLink 映射在 `serviceMeta.ts` 或新增 helper：

```tsx
function ResultToolbar({ result, copied, onCopied }) {
  const t = useT();
  const link = serviceWordLink(result.service_id, result); // 见下
  return (
    <div className="flex items-center gap-1 border-t border-border pt-2">
      <AudioButton audioKey={`result:${result.service_id}:${result.audio_url ?? ""}`} className="icon-btn btn-ghost !h-7 !w-7" url={result.audio_url ?? null} />
      <button className="icon-btn btn-ghost !h-7 !w-7" onClick={async () => { await api.copyToClipboard(result.text); onCopied(); }} title={copied ? t("common-copied", null, "Copied") : t("common-copy", null, "Copy")}>
        {copied ? <Check size={15} /> : <Copy size={15} />}
      </button>
      {link && (
        <a className="icon-btn btn-ghost !h-7 !w-7" href={link} target="_blank" rel="noreferrer" title={t("main-open-link", null, "Open link")}>
          <ExternalLink size={15} />
        </a>
      )}
    </div>
  );
}
```

`serviceWordLink`：Youdao→`https://dict.youdao.com/w/<text>`、Bing→`https://cn.bing.com/dict/search?q=<text>`、Google→`https://translate.google.com/?sl=<from>&tl=<to>&text=<text>`、DeepL→`https://www.deepl.com/translator`、OpenAI→null。`text` 用源词（由 `ResultBody` 透传 `sourceText`，或回退 `result.text`）。

- [ ] **Step 5: 验证编译**

Run: `npm --prefix ui run typecheck && npm --prefix ui run build`
Expected: PASS

- [ ] **Step 6: 运行时核对**（用 `run`/`verify` 技能启动应用）— 输入英文单词如 `good`，确认 Youdao 卡片显示：大词头 `good`、译文、US/UK 音标、考试标签、多词性释义（adj./n./adv. 各一行）、词形变化（复数/比较级/最高级）、网络短语。与 EasyDict 截图比对。

- [ ] **Step 7: 提交**

```bash
git add ui/src/App.tsx
git commit -m "feat(ui): render source dictionary with full EasyDict block order"
```

### Task 5: Phase 2 — Youdao 后端核对

**Files:**
- Read/verify: `crates/core/src/services/youdao.rs`（`dictionary_from_official`、`dictionary_from_web_dict`、`append_ec_dictionary` 等）

- [ ] **Step 1: 核对** — 用 `vendor/easydict/.../Youdao/Model/DictJSONExample/v4/good_v4.json` 比对 youdao.rs 解析，确认 source_dictionary 覆盖：phonetics(US/UK+audio)、parts(pos+tran)、exchanges(wfs 按「或」拆分)、simple_words(web_trans + ce)、tags(exam_type)。若有缺失（如 synonyms/antonyms/collocation/etymology——Youdao V4 本就不提供，留空即可）记录。

- [ ] **Step 2: 若 youdao.rs 未填 `extra` raw** — 可选：在 `translate_official`/`translate_web` 设 `extra: Some(serde_json::to_value(&parsed).ok())` 便于调试。非必须。

- [ ] **Step 3: 验证** — `cargo test -p translator-core`（已有 youdao wiremock 测试须通过）。

- [ ] **Step 4: 提交**（若有改动）

---

## Phase 3 — Bing 字典路径

> 执行前先通读 `crates/core/src/services/bing.rs` 现状（host/token/IG/IID 抓取、ttranslatev3 调用）与 `vendor/easydict/.../Bing/BingService+Translate.swift`、`BingRequest.swift`、`BingLookupResponse.swift`。TDD：用 EasyDict 文档的响应结构构造夹具。

### Task 6: Bing v7 dictionary-words 请求 + 解析

**Files:**
- Modify: `crates/core/src/services/bing.rs`（新增 `BingDictResponse` 结构 + `parse_bing_dict` + 请求函数）
- Test: `crates/core/src/services/bing.rs` 内 `#[cfg(test)]`（wiremock 或纯解析单测）

**Interfaces:**
- Produces: `fn parse_bing_dict(json: &serde_json::Value) -> DictionaryResult`，填充 phonetics/parts/exchanges/simple_words/synonyms/antonyms/collocation。

- [ ] **Step 1: 写失败测试** — 构造 `good` 的 v7 响应夹具（`value[0].meaningGroups[]`，含 `发音`/`快速释义`/`词组`/`分类词典`/`搭配`/`变形` 各类），断言解析出 US/UK phonetics、parts（按 POS）、synonyms/antonyms、collocation、exchanges。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p translator-core --lib bing`
Expected: FAIL

- [ ] **Step 3: 实现** — 新增 `BingDictResponse` 反序列化 + `parse_bing_dict`，按 `partsOfSpeech[0].description` 分派（照搬 EasyDict `parseBingDictTranslate`）：`发音`→phonetics（US/UK，audioUrl.contentUrl，UK 的 `tom`→`george`）、`快速释义`→parts、`词组`→simple_words、`分类词典`→synonyms/antonyms、`搭配`→collocation、`变形`→exchanges。POS 经 `part_abbreviation`。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p translator-core --lib bing`
Expected: PASS

- [ ] **Step 5: 提交**

### Task 7: Bing tlookupv3 lookup + 接入 translate 流程

**Files:**
- Modify: `crates/core/src/services/bing.rs`

- [ ] **Step 1: 写失败测试** — `tlookupv3` 响应夹具（`translations[].posTag/displayTarget/backTranslations`），断言 en→zh 产出 parts、zh→en 产出 simple_words；transliteration→phonetic。

- [ ] **Step 2: 运行确认失败**

- [ ] **Step 3: 实现** — 新增 `BingLookupResponse` + `parse_bing_lookup`；在 `translate` 中：en 单词→zh 时额外发 v7 dict 请求（`isEnglishWordToChinese` 判定，对齐 EasyDict：`from==en && to==zh-Hans && wordCount==1`），其余情形随 translate 发 tlookupv3。把解析结果填入 `source_dictionary`，`text` 仍取 ttranslatev3 译文（dict 路径下取首个 part 首个 mean 作 text，对齐 EasyDict）。复用现有 host/token；205→reset+重试1次。

- [ ] **Step 4: 运行确认通过** — `cargo test -p translator-core --lib bing`

- [ ] **Step 5: 提交**

---

## Phase 4 — Google webApp + tk 签名

> 执行前通读 `crates/core/src/services/google.rs` 现状与 `vendor/easydict/.../Google/GoogleService+Translate.swift`、`google-translate-sign.js`。

### Task 8: 移植 Google `tk` 签名到 Rust

**Files:**
- Create: `crates/core/src/services/google_sign.rs`（或并入 google.rs）
- Modify: `crates/core/src/services/mod.rs` 不动（google.rs 内部模块）
- Test: `crates/core/src/services/google.rs` 内 `#[cfg(test)]`

**Interfaces:**
- Produces: `pub(crate) fn google_tk(text: &str, tkk: &str) -> String`。

- [ ] **Step 1: 用 EasyDict JS 作 oracle 写失败测试** — `node -e` 跑 `vendor/easydict/.../google-translate-sign.js` 的 `sign("good")`（TKK 固定 `444000.1270171236`）得到期望 tk；Rust 测试断言 `google_tk("good", "444000.1270171236")` 等于该值（多取几组样本）。

- [ ] **Step 2: 运行确认失败**

- [ ] **Step 3: 实现** — 移植 `sign()`：UTF-8 字节数组 → `xr` 混淆 → `mod 1e6`，与 `TKK` 第二段组合。照搬 JS 逻辑逐行翻译。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p translator-core --lib google`
Expected: PASS（与 node oracle 完全一致）

- [ ] **Step 5: 提交**

### Task 9: Google webApp 请求 + dict 块解析 + 接入

**Files:**
- Modify: `crates/core/src/services/google.rs`

- [ ] **Step 1: 写失败测试** — webApp 响应夹具（数组：`[0]` 译文段、`[0][1][3]` 音标、`[1]` 字典块 `[[POS,[means...]]]`、`[2]` 检测语言），断言 en→zh 产出 parts+phonetic、zh→en 产出 simple_words。

- [ ] **Step 2: 运行确认失败**

- [ ] **Step 3: 实现** — 新增 `webapp_translate`：`GET translate_a/single?client=webapp&dt=at&dt=bd&dt=ex&dt=ld&dt=md&dt=qca&dt=rw&dt=rm&dt=ss&dt=t&sl=&tl=&hl=en&...&tk=&q=`；TKK 从 `translate.google.com` 首页 HTML 正则 `tkk:'\d+\.\d+'` 刷新（带新鲜度判定：首段 vs `now/3600`）。解析 `responseArray[1]`→parts/simpleWords、`[0][1][3]`→phonetic、`[2]`→detected_source、`[0]`→text。webApp 无字典数据时回退现有 GTX。

- [ ] **Step 4: 运行确认通过** — `cargo test -p translator-core --lib google`

- [ ] **Step 5: 提交**

---

## Phase 5 — DeepL alternatives

> 执行前通读 `crates/core/src/services/deepl.rs` 的 web jsonrpc 路径与 `vendor/easydict/.../DeepL/DeepLTranslateResponse.swift`。

### Task 10: DeepL 捕获 alternatives

**Files:**
- Modify: `crates/core/src/services/deepl.rs`（`DeepLTranslateResponse`/`DeepLTranslateText` 加 `alternatives` 字段 + web 路径解析）

- [ ] **Step 1: 写失败测试** — web 响应夹具 `result.texts[0].alternatives=[{text:"good"},{text:"fine"}]`，断言 `TranslateResult.alternatives == ["good","fine"]`，`text` 仍为 `texts[0].text`。

- [ ] **Step 2: 运行确认失败**

Run: `cargo test -p translator-core --lib deepl`
Expected: FAIL

- [ ] **Step 3: 实现** — `DeepLTranslateText` 增 `#[serde(default)] alternatives: Vec<DeepLAlternative>`（`{ text: String }`）；web 路径 `parse_web_translate_response` 把 `alternatives.iter().map(|a| a.text).collect()` 填入 `result.alternatives`。官方 API 路径无 alternatives，留空。

- [ ] **Step 4: 运行确认通过**

Run: `cargo test -p translator-core --lib deepl`
Expected: PASS

- [ ] **Step 5: 提交**

```bash
git add crates/core/src/services/deepl.rs
git commit -m "feat(deepl): capture web alternatives"
```

---

## 收尾验证

- [ ] `cargo test --workspace` 全绿
- [ ] `npm --prefix ui run typecheck && npm --prefix ui run build && npm --prefix ui run locales:check` 全绿
- [ ] 启动应用，输入 `good`：Youdao/Bing/Google 多区块显示，DeepL 显示备选，OpenAI 单结果；输入中文词 `美`：zh→en 的 simple_words/pinyin 显示；输入句子：仅译文。
- [ ] 与 EasyDict 截图比对排版顺序与区块。
- [ ] `git log feat/easydict-result-replication` 检查提交序列。
