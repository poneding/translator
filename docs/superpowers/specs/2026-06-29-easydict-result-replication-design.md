# 复刻 EasyDict 翻译结果（多结果/字典）展示 — 设计文档

- 日期: 2026-06-29
- 状态: 已批准（设计阶段），待 spec 复核
- 范围: Translator 应用（Tauri + Rust + React/TS），参考 `vendor/easydict`

## 1. 背景与目标

用户反馈：输入一个英文单词时，应用只显示一个翻译结果。而参考实现 EasyDict 在同样输入下会给出多个翻译结果（多词性释义、网络释义、音标、词形变化等）以及其他细节。

目标：**完全参考 `vendor/easydict` 各翻译 API 获取结果并呈现的逻辑，在 Translator 应用中 1:1 复刻翻译结果的数据与展示**（聚焦结果内容区块，不含卡片外壳 chrome）。

## 2. 当前状态与根因（调研结论）

### 2.1 关键洞察
**Youdao 后端其实已经把多结果数据解析好了，只是前端没渲染出来。**

- `crates/core/src/services/youdao.rs` 已调用有道 dict 接口，把音标(phonetics)、词性释义(parts)、词形变化(exchanges)、网络释义(simple_words)、考试标签(tags) 解析进 `source_dictionary`，逻辑与 EasyDict V4 基本一致（含已有的 `part_abbreviation` 助手）。
- 但前端 `ui/src/App.tsx` 的 `ResultBody`（line 1057）只渲染 `result.target_dictionary`，完全忽略 `source_dictionary`/`dictionary`。`target_dictionary` 仅在目标是英文短词时才被 Youdao 填充。因此最常见的「英文单词→中文」场景下，Youdao 已解析好的多词性释义在卡片里不可见 —— 这就是「只有一个翻译结果」的主因，且修复成本最低。

### 2.2 各服务多结果来源对照（EasyDict vs 当前应用）

| 服务 | EasyDict 多结果来源 | 当前应用 |
|---|---|---|
| Youdao | dict V4 `ec.word.trs[]`(每词性一条) + `web_trans`(网络释义) + `ec.word.wfs[]`(词形) + 音标 + `exam_type`(标签) | 后端已解析；前端未渲染 source 侧 |
| Bing | v7 `dictionarywords/search`(丰富：释义/词形/同反义词/搭配) + `tlookupv3` lookup | 只取单个 text，未调字典接口 |
| Google | webApp `translate_a/single` 带 `dt=bd` 等，en↔zh 产出 parts/simpleWords/音标 | 只取单 text |
| DeepL | 无字典模式；web jsonrpc 返回最多 3 个 `alternatives`，EasyDict 自己丢弃 | 同样丢弃 |
| OpenAI | 无字典 API，EasyDict 当纯翻译 | 一致（单结果） |

### 2.3 EasyDict 渲染顺序（`EZWordResultView.refreshWithResult`，自上而下）
大词头 → 译文/错误 → 音标 → 标签 → 词性释义(parts，每词性一行，means 用 `; ` 连接) → 词形(exchanges) → 同义词 → 反义词 → 搭配 → 网络短语(simple_words) → 词源(etymology) → 底部工具栏(音频/复制/链接)。UI 不区分词/句，服务填什么字段就渲染什么。

### 2.4 数据模型差距
`DictionaryResult` 缺 `synonyms/antonyms/collocation/etymology`；`TranslateResult.text` 是单字符串、无 `alternatives` 字段；12 个 `.ftl` 文件缺字典区块标题的本地化串。

## 3. 范围（已与用户确认）

- **服务范围**：仅现有 5 个服务（Youdao/Bing/Google/DeepL/OpenAI），不新增服务。
- **显示范围**：完整 1:1，复刻 EasyDict 渲染的全部区块（含 synonyms/antonyms/collocation/etymology）。
- **DeepL**：展示备选译文（alternatives）—— 对 EasyDict 的小幅、合理偏离，以更好满足「多结果」诉求。
- **OpenAI**：保持单结果，与 EasyDict 一致。
- **不在范围**：卡片外壳 chrome（折叠/箭头/重试/stop/markdown 切换等）；HTML 字典（AppleDictionary/MDict）；OCR；流式 AI 的 markdown 渲染。

## 4. 方案选择

**方案 A（采用）**：在现有模型上扩展字段 + 各服务补全解析 + 前端重写渲染。复用已在用的 `TranslateResult`/`DictionaryResult`，最小破坏。

- 方案 B（引入 `QueryType` 能力位掩码 + 独立 dict/translate 路径）：更忠实 EasyDict 架构，但需大改 `TranslationService` trait 与 coordinator，收益边际，YAGNI。
- 方案 C（模型整体重写为 `EZQueryResult` 镜像）：最忠实，但波及 IPC/历史/自动复制/流式，高风险。

## 5. 设计

### 5.1 数据模型 (`crates/core/src/model.rs` + `ui/src/types/bindings.ts`)

- `TranslateResult` 增字段：
  - `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub alternatives: Vec<String>` —— DeepL 备选译文。
- `DictionaryResult` 增字段（均 `#[serde(default, skip_serializing_if = ...)]`）：
  - `pub synonyms: Vec<DictionaryPart>` —— 同义词（按词性分组）。
  - `pub antonyms: Vec<DictionaryPart>` —— 反义词。
  - `pub collocation: Vec<DictionaryPart>` —— 搭配。
  - `pub etymology: Option<String>` —— 词源。
- `DictionaryResult::is_empty()` 纳入新字段判断。
- 抽取 `part_abbreviation` 为 core 公共函数（当前在 youdao.rs 局部实现），供 Bing/Google 复用。映射表照搬 EasyDict（`noun→n.`、`verb→v.`、`形容词→adj.`、`adverb→adv.` 等；已缩写形式原样通过）。
- `bindings.ts` 同步以上新字段（`TranslateResult.alternatives?`、`DictionaryResult.synonyms?/antonyms?/collocation?/etymology?`）。

### 5.2 后端各服务

- **Youdao (`youdao.rs`)**：后端已完成。仅核对 source_dictionary 各字段齐全（已确认 phonetics/parts/exchanges/simple_words/tags）。`extra` 可选填 raw 响应便于调试。无实质性新工作。
- **Bing (`bing.rs`)**（最大块）：
  - 新增字典路径：en 单词→zh 时调 v7 `GET https://<host>/api/v7/dictionarywords/search?q=<text>&...`，解析 `value[0].meaningGroups[]`，按 `partsOfSpeech[0].description` 分派：`发音`→phonetics、`快速释义`→parts、`词组`→simple_words、`分类词典`→synonyms/antonyms、`搭配`→collocation、`变形`→exchanges。
  - 随 translate 调 `tlookupv3`：en→zh 产出 parts（按 posTag 分组），zh→en 产出 simpleWords；transliteration 作音标。
  - 复用现有 host 发现 / IG / IID / token 抓取逻辑；205 状态触发 token 重置 + 一次重试。
- **Google (`google.rs`)**：
  - 新增 webApp 路径：`GET https://translate.google.com/translate_a/single?client=webapp&dt=at&dt=bd&dt=ex&dt=ld&dt=md&dt=qca&dt=rw&dt=rm&dt=ss&dt=t&sl=<from>&tl=<to>&hl=en&...&tk=<sign>&q=<text>`。
  - 把 `vendor/easydict/.../google-translate-sign.js` 的 `tk` 算法移植到 Rust（UTF-8 字节 → xr 混淆 → mod 1e6），含从 `translate.google.com` 首页 HTML 正则刷新 `TKK`。
  - 解析响应数组：`[1]`→字典块（en→zh parts / zh→en simpleWords）、`[0][1][3]`→音标、`[2]`→检测语言、`[0]`→译文段。
  - 现有 GTX 路径保留为回退（webApp 无字典数据时）。
- **DeepL (`deepl.rs`)**：web jsonrpc 路径解析 `result.texts[0].alternatives[].text` → `TranslateResult.alternatives`（已发 `requestAlternatives:3`，当前丢弃）。
- **OpenAI (`openai.rs`)**：不变。

### 5.3 前端渲染 (`ui/src/App.tsx`)

- `ResultBody`：卡片内渲染的字典块取 `result.source_dictionary ?? result.dictionary`（与 EasyDict 单一 `wordResult` 对应）。`result.target_dictionary` 不作为独立块渲染，其音标音频已通过 `result.audio_url` 透传到底部工具栏；仅当 source/generic 均缺失时才回退到 target_dictionary 作为兜底。
- `DictionaryDetails` 扩展并按 EasyDict 顺序重排为：**大词头 → 音标 → 标签 → 词性释义 → 词形 → 同义词 → 反义词 → 搭配 → 网络短语 → 词源**（当前顺序 phonetics→tags→parts→simpleWords→exchanges，需把 exchanges 提到 simpleWords 之前并追加新区块）。
- 大词头：`isShortWord(源词) && hasDictionary` 时以大字号显示源词（英文 ≤20 字符、其他语言 ≤7 字符视为短词，照搬 EasyDict `EZEnglishWordMaxLength` 等）。
- alternatives 区块：当 `result.alternatives` 非空时，在译文下方以列表展示（DeepL）。
- 底部工具栏：把当前内联的 audio/copy 收进卡片底部工具栏，并新增「链接」按钮（打开服务 wordLink，如 Youdao/Bing/Google 的词典页）。音频播放 `result.text`。
- 保留 `SourceAudioControls`（源编辑器共享源发音按钮）；卡片内各服务自带音标行（与 EasyDict 一致）。
- 复用现有 `phoneticDisplayLabel`（US→美、UK→英、Pinyin→拼音）。

### 5.4 i18n（12 个 `.ftl`：ar/de/en/es/fr/it/ja/ko/pt/ru/zh-Hans/zh-Hant）

新增 key：`synonyms`、`antonyms`、`collocation`、`etymology`、`alternatives`。（大词头、词性缩写来自数据本身，无需标签。）各语言均需翻译。

## 6. 分阶段执行

每阶段独立可验证、可见价值。Phase 1 单独即修复主诉（Youdao 多结果可见）。

- **Phase 1**：模型扩展（alternatives/synonyms/antonyms/collocation/etymology）+ `part_abbreviation` 提取 + 前端重写（ResultBody/DictionaryDetails/大词头/底部工具栏）+ i18n。→ Youdao 立即显示多词性释义。
- **Phase 2**：Youdao 核对/补齐（小）。
- **Phase 3**：Bing 字典路径（v7 dict + tlookupv3）。
- **Phase 4**：Google webApp + tk 签名移植。
- **Phase 5**：DeepL alternatives 捕获。

建议在 Phase 1 完成后设一个验收检查点（用户可先行确认 Youdao 多结果效果），再继续 Phase 3-5。

## 7. 测试

- 后端：扩展现有 wiremock 测试（`crates/core` 已有 youdao/bing 测试基础），为 Bing v7 dict、Google webApp、DeepL alternatives 增加固定响应夹具与解析断言。使用 `vendor/easydict/.../Youdao/Model/DictJSONExample/v4/*.json` 作为 Youdao 夹具参考。
- 前端：TypeScript 编译通过（`tsc`/`vite build`）；人工/截图核对 Youdao 英文单词→中文 的多区块渲染与 EasyDict 一致。
- 回归：en→zh 单词、zh→en 单词、句子翻译、DeepL 备选、OpenAI 单结果。

## 8. 风险与开放问题

- **Bing/Google 字典路径仅在 en↔zh 生效**（EasyDict 行为一致）。其他语对仍为单结果——属预期，非缺陷。
- **Google `tk` 签名**：算法可能随 Google 更新失效；TKK 刷新逻辑需照搬。风险中等。
- **Bing token/IG/IID 抓取**依赖 HTML 正则，易随页面变动失效；复用现有逻辑并保留 205 重试。
- **大词头/底部工具栏**为视觉结构变更，需确保不破坏现有布局与暗色模式。
- 开放：是否在 Phase 1 后即验收（建议是）；若用户希望一次性全做，则连续推进 Phase 3-5。

## 9. 参考文件索引

EasyDict（参考源）：
- `vendor/easydict/Easydict/Swift/Service/Youdao/YoudaoService+Translate.swift` / `+Dict.swift` / `EZQueryResult+DictV4.swift` / `Model/YoudaoDictResponseV4.swift`
- `vendor/easydict/Easydict/Swift/Service/Bing/BingService+Translate.swift` / `BingRequest.swift` / `BingLookupResponse.swift`
- `vendor/easydict/Easydict/Swift/Service/Google/GoogleService+Translate.swift` / `google-translate-sign.js`
- `vendor/easydict/Easydict/Swift/Service/DeepL/DeepLService+Translate.swift` / `DeepLTranslateResponse.swift`
- `vendor/easydict/Easydict/Swift/Service/Model/QueryResult.swift`（`EZTranslateWordResult`/`EZTranslatePart`/`EZTranslateSimpleWord`/`EZWordPhonetic`/`EZTranslateExchange` + `partAbbreviation`）
- `vendor/easydict/Easydict/objc/ViewController/View/WordResultView/EZWordResultView.m`（核心渲染例程）
- `vendor/easydict/Easydict/Swift/Utility/Extensions/String/String+Analysis.swift`（`shouldQueryDictionary`/`isShortWordLength`）

当前应用（修改目标）：
- `crates/core/src/model.rs`、`crates/core/src/services/{youdao,bing,google,deepl,openai}.rs`、`crates/core/src/services/mod.rs`
- `ui/src/App.tsx`（`ResultBody`/`DictionaryDetails`/`SourceAudioControls`/`ResultCard`）、`ui/src/types/bindings.ts`、`ui/src/locales/*.ftl`
