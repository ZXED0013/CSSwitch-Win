# CSSwitch 产品与 Claude Science 能力地图

状态：当前能力/所有权正文

最后复核：2026-08-14（Asia/Taipei）

失效条件：官方 Science capability surface、CSSwitch production owner、证据词表、当前 source、
Science package/version 或任一 artifact/runtime tuple 改变时，受影响行立即复核。

本文是 Claude Science 能力范围与 CSSwitch 第三方模式托管决策的唯一当前表。
它回答五个问题：

1. 这项能力是否由 Claude Science 官方产品拥有；
2. CSSwitch 第三方模式是否必须托管或只做窄桥接；
3. 哪些管理面明确是 CSSwitch 的 non-target；
4. 即使能力语义不归 CSSwitch，第三方模式仍需保证哪一层兼容路径；
5. 当前已知到哪一层，还有什么是 `UNKNOWN`。

内部依赖和故障归属见
[Claude Science 能力依赖](../architecture/science-capability-dependencies.md)。
exact HEAD、版本、package、URL、hash、route 与日期化调查只留在
[审计/证据索引](../audits/README.md)。2026-07-30 的
[架构调研](../audits/2026-07-30-v084-architecture-reconnaissance.md)只绑定当时的 official/package
crosswalk；当前 source 与版本差异见
[2026-08-14 全仓审计](../audits/2026-08-14-next-repository-docs-architecture-review.md)。
本表不是 release PASS，也不把 source、test、package-static 或 fixture 写成 live。
本表中的 `non-target` 是当前第三方模式托管决策，不是永久产品禁令。受管
Skill/MCP/Plugin 扩展控制面的目标合同已经冻结，源码现有一个无产品 caller 的
inspect-only adapter；它不是 plan、apply、artifact 或 runtime 能力，仍按下表的当前结论
报告。规划意图和 runtime-orphan source 都不是当前用户能力证据。

## 判定词

### 第三方模式处理

| 判定 | 含义 |
|---|---|
| `必须托管` | 没有这层，CSSwitch 第三方模式不能安全、稳定地成立 |
| `窄桥接` | CSSwitch 只提供明确入口或适配，不接管 Science 的领域语义 |
| `原生保留` | 由 Science 继续管理；CSSwitch 只保证隔离、启动或状态保全 |
| `不托管` | 由用户、账号或外部服务管理 |

### 兼容责任与运行路径

所有权与兼容责任是两个不同判断。“Science 官方拥有”不能直接推出
“CSSwitch 无事可做”；“CSSwitch 不托管”也不能直接推出“第三方模式不支持”。
一项能力通常由多个 operation/stage 组成；每个 stage 必须落入下面一类，再按
实际调用顺序组合，不能把整项能力压成单一枚举：

| stage 类别 | CSSwitch 必须保证 | CSSwitch 不拥有 |
|---|---|---|
| `CSSWITCH-RUNTIME` | executable、隔离布局、managed identity、启动/停止/恢复、protected projection、状态一致性与诊断 | Science data-dir 内的产品语义和外部 runtime 内部实现 |
| `MODEL-GATEWAY` | provider/profile/model 选择，Anthropic/OpenAI 协议适配，stream、tool call/result、`tool_choice`、reasoning、structured output、vision 与稳定错误/停止语义的声明式兼容或明确降级 | Science 的 Agent/plan/Reviewer 语义，第三方模型服务、配额、计费和实际质量 |
| `SCIENCE-NATIVE` | 隔离 HOME/data-dir、runtime identity、持久化与恢复不能破坏原生状态；必要的端口、路径和进程包络保持可用 | project/session、artifact、permission、memory、environment/kernel、Skill load/trigger 等领域语义 |
| `NARROW-BRIDGE` | 只对显式启用的入口负责；固定输入/输出、身份、权限、生命周期、重启/清理和局部失败降级 | bridge 之后的 Agent、OpenSSH/server、Codex/provider、MCP tool 或 Skill domain execution |
| `SCIENCE-EXTERNAL` | 把 Science-native client 与外部服务的语义依赖单列；不把 transport tunnel 冒充上层能力，隔离或 network policy 的影响按流量面报告 | connector、文献、云、remote MCP、远端计算的凭证、账号、服务和费用 |
| `OFFICIAL-ENTITLEMENT` | 明确显示为官方账号/组织能力，不用虚拟登录或第三方模型结果冒充可用 | Claude OAuth、官方 catalog、usage、subscription、hosted connector、Reviewer entitlement |

stage 链描述语义/责任，不单独决定 socket transport。当前非 loopback HTTPS
还可能经进程级 `HTTPS_PROXY` 进入 Gateway raw `CONNECT`；这只增加 tunnel
policy 与 transport diagnostics，不把该 stage 改成 `MODEL-GATEWAY`。

这张 stage 表不是第二张逐能力支持矩阵。下方唯一决策表继续保存逐能力
ownership、典型 stage 链、托管/non-target、证据与 `UNKNOWN`；组合规则、transport
和故障归属的稳定解释只在
[能力依赖](../architecture/science-capability-dependencies.md)维护。

### 已知证据层

| 层级 | 只能说明什么 |
|---|---|
| `OFFICIAL` | 官方公开 capability surface 或 ownership；未另注日期的行绑定 [2026-07-30 架构调研 §7](../audits/2026-07-30-v084-architecture-reconnaissance.md#7-官方资料与-0125-crosswalk) 的 URL/访问日 crosswalk，显式标注 `2026-08-14 audit` 的行绑定当日官方 changelog 复核；两者都不是无限期官方合同 |
| `SOURCE` | 当前 CSSwitch 源码中的实现边界 |
| `TEST` | 对应 source contract 的测试结果 |
| `PACKAGE-STATIC` | 指定 Science package 中存在的静态表面 |
| `FIXTURE` | 隔离 mock/fake/fixture 中的行为；不得写成真实服务或 current live |
| `ARTIFACT` | 绑定 exact source 的包内身份与静态组成；不证明 executable 已运行 |
| `ISOLATED-LIVE` | 绑定 exact artifact/runtime 的隔离运行观察；必须在表内标出 scope，不外推真实账号、provider、Skill、installed 或 release |
| `INSTALLED-LIVE` | 绑定已安装 artifact 与 runtime 身份的运行观察 |
| `AUTHORIZED-LIVE` | 用户对明确 provider/model/账号/Skill/SSH capability 单独授权后，在 exact artifact/runtime 上取得的真实结果；只覆盖该 operation/scope |

`OFFICIAL` 不证明指定账号、版本或第三方模式可用，也不能在官方文档改版后自动保持真
实；需要更新 ownership 时先写日期化 audit，再改本表。`SOURCE/TEST` 不证明最终
artifact；`PACKAGE-STATIC` 不证明实际调用；`FIXTURE` 不证明 live。
表内未附 tuple 的 `SOURCE` / `TEST` 只表示当前 source surface 与对应测试合同存在；是否在 current
exact HEAD 执行并通过，必须从[已验证状态](../../.agents/context/verified-state.md)读取；不能把表中
标签读成 current PASS。
本表中的 `ARTIFACT` / `ISOLATED-LIVE` 条目首先绑定
[B-RUNTIME-01](../evidence/investigations/2026-08-11-claude-science-0.1.25-b-runtime-01.md)、
[B-CORE-01](../evidence/investigations/2026-08-11-claude-science-0.1.25-a60c2ee-b-core-01.md)、
[B-CONTEXT-01](../evidence/investigations/2026-08-11-claude-science-0.1.25-a60c2ee-b-context-01.md) 与
[B-PROVIDER-01](../evidence/investigations/2026-08-11-claude-science-0.1.25-a60c2ee-b-provider-01.md)
各自写明的 exact tuple 和 scope；另有
[RM-46 Provider 配置 UX](../evidence/investigations/2026-08-11-rm46-provider-configuration-ux.md)
只绑定其 exact artifact 的本地 mock UI scope。后续
[2026-08-12 日期化验收](../evidence/investigations/2026-08-12-csswitch-18a67881-adoption-installed-live-acceptance.md)
独立绑定 Science 0.1.25 adoption、normal installed artifact 与逐 operation
`AUTHORIZED-LIVE` 的 PASS / INCONCLUSIVE / NOT-RUN；它不继承给当前 source。

2026-08-14 的[日期化审计](../audits/2026-08-14-next-repository-docs-architecture-review.md)记录：
[官方 Science changelog](https://claude.com/docs/claude-science/changelog.md)已进入比历史 live tuple 更新的
版本。当前 source、官方 current package 与历史 artifact/runtime 之间的 compatibility 证据不能继承；
当前判定只从[已验证状态](../../.agents/context/verified-state.md)读取。
当日 changelog 新增或改变的首会话环境自修复、session star/notification、per-session reasoning effort 和
usage-credit 管理，按本表既有 ownership 规则推断，分别归入安装、`SCIENCE-NATIVE`、
`SCIENCE-NATIVE → MODEL-GATEWAY` 与 `OFFICIAL-ENTITLEMENT` 域；这项分类是 CSSwitch 架构映射，
不是官方原文，也没有产生新的 CSSwitch owner。第三方模式兼容性仍未验证。

## 能力、所有权与托管决策表

`UNKNOWN` 单列记录尚未建立的事实。空白不表示已验证。

| 能力域 | 能力 | Science 官方拥有 | 典型 stage 链 | CSSwitch 第三方模式处理 | CSSwitch non-target | 已知证据层 | UNKNOWN |
|---|---|---|---|---|---|---|---|
| 安装与平台 | macOS App/CLI、首会话环境修复 | 是：Science App、CLI 与本地数据语义 | `CSSWITCH-RUNTIME → SCIENCE-NATIVE` | `原生保留`；第三方受管启动另见“第三方运行包络” | 否 | `OFFICIAL`（2026-08-14 audit）、历史 `PACKAGE-STATIC` | 当前 final artifact、普通 installed/repair 行为与发布态 |
| 安装与平台 | whole-app remote Linux / WSL | 是：Science 的整机部署能力 | `SCIENCE-NATIVE → SCIENCE-EXTERNAL` | `原生保留`；CSSwitch 当前不提供部署管理 | 是：CSSwitch 不提供 whole-app Linux/WSL 管理面 | `OFFICIAL` | 第三方模式兼容性、端口、preview 与数据目录行为 |
| 第三方运行包络 | 隔离 HOME、持久 data-dir、runtime identity、启动/停止/恢复 | 否：这是 CSSwitch 第三方模式责任 | `CSSWITCH-RUNTIME` | `必须托管` | 否 | `SOURCE`、`TEST`；历史 `ARTIFACT` / `ISOLATED-LIVE`（B-RUNTIME 与 adoption evidence）及 `INSTALLED-LIVE` | 当前 exact artifact/runtime、官方 current Science compatibility 与 installed 连续性 |
| 第三方运行包络 | 本地虚拟登录、loopback Gateway 与受限 route | 否：这是 CSSwitch 第三方模式责任 | `CSSWITCH-RUNTIME → MODEL-GATEWAY` | `必须托管`；虚拟登录只建立本地受管路径 | 否 | `SOURCE`、`TEST`；历史 `ARTIFACT` / `ISOLATED-LIVE`（B-RUNTIME/B-CORE/B-CONTEXT 与 adoption evidence）及 `INSTALLED-LIVE` | 当前 exact artifact、真实第三方 provider/model 与官方 current Science compatibility |
| Provider | profile、第三方 provider、模型 selector、模型目录与协议适配 | 否：第三方 provider 拥有模型、认证、配额和计费 | `MODEL-GATEWAY`；provider 只作为 external dependency | `必须托管` profile/selector/routing；不拥有模型服务 | 否 | `SOURCE`、`TEST`；历史 `ARTIFACT` / `ISOLATED-LIVE`（protocol/configuration local-mock）与 `AUTHORIZED-LIVE`（逐 operation 分项） | 当前 Desktop E2E、未闭合 provider/model operation、账号、配额、计费与服务质量 |
| Project | project、session、conversation、custom instructions、archive/import、star 与通知 | 是：Science 本地 UI、数据库和 session control plane | `SCIENCE-NATIVE`；推理操作再进入 `MODEL-GATEWAY` | `原生保留`；CSSwitch 只隔离和保全数据域 | 是：不提供 Science 语义 CRUD/notification 管理 | `OFFICIAL`（2026-08-14 audit）、历史 `PACKAGE-STATIC` / `ISOLATED-LIVE`（B-CONTEXT local surface/request-shape scope） | current star/notification、archive/unarchive、import、restart readback 与完整语义 |
| Agent 工作流 | plans、approval、delegation、fork/恢复与 per-session reasoning effort | 是：Science session/plan control plane | `SCIENCE-NATIVE`；只有创建、继续或执行推理的 operation 再进入 `MODEL-GATEWAY`，本地恢复/readback 不进入 | `原生保留` | 是：不实现第二套 plan/delegation engine | `OFFICIAL`（2026-08-14 audit）、历史 `PACKAGE-STATIC` / `ISOLATED-LIVE`（B-CONTEXT surface/request-shape scope） | per-session reasoning 的第三方 payload 映射、服务端结果、配额和 entitlement |
| 文件与权限 | attachments、路径授权、standing grant、撤销和越界拒绝 | 是：Science 拥有 permission UI、scope、持久化和 enforcement；用户拥有授权决定与资源 | `SCIENCE-NATIVE`；模型 payload 再进入 `MODEL-GATEWAY` | `原生保留`；CSSwitch 不扩大授权 | 是：不提供通用文件权限管理 | `OFFICIAL`、`PACKAGE-STATIC`、`ISOLATED-LIVE`（B-CORE grant/revoke/sibling-boundary scope） | 跨 session、重启后 enforcement 与真实用户资源 |
| 产物 | artifact、版本、diff、preview、execution/environment/review provenance | 是：Science artifact/execution store | `SCIENCE-NATIVE`；生成操作可进入 `MODEL-GATEWAY` | `原生保留` | 是：不重建 artifact/provenance 系统 | `OFFICIAL`、`PACKAGE-STATIC`、`ISOLATED-LIVE`（B-CORE version/diff/lineage scope） | download/delete/restart 与完整 execution/environment/review provenance |
| 上下文 | annotations | 是：Science message/context state | `SCIENCE-NATIVE`；进入请求时再走 `MODEL-GATEWAY` | `原生保留` | 是：不实现第二套 annotation 管理面 | `OFFICIAL`、`PACKAGE-STATIC`、`ISOLATED-LIVE`（B-CORE Markdown selection scope） | 格式覆盖、定位稳定性、批量提交与持久化 |
| 上下文 | project memory、search、context compaction | 是：Science project/session context | `SCIENCE-NATIVE`；进入请求时再走 `MODEL-GATEWAY` | `原生保留` | 是：不实现第二套 memory engine | `OFFICIAL`、`PACKAGE-STATIC`、`ISOLATED-LIVE`（B-CONTEXT surface/request-shape scope） | 保存时点、跨 session 隔离、删除语义与服务端结果 |
| 计算环境 | starter/task Conda、Python/R kernel、包安装与 compute monitor | 是：Science 环境与 kernel 生命周期 | `SCIENCE-NATIVE`；远程 package registry/mirror 获取再进入 `SCIENCE-EXTERNAL` | `原生保留`；CSSwitch 只保护 Science-owned opaque roots 的边界 | 是：不重写 Conda/kernel 生命周期 | `OFFICIAL`、`PACKAGE-STATIC`、`SOURCE` | 第三方 current live、task env reuse、kernel/persistent package 边界 |
| 计算环境 | Node environment | 未确认 | `SCIENCE-NATIVE`（owner 仍 `UNKNOWN`） | `不托管`；在 ownership 明确前不扩 CSSwitch 范围 | 是：当前不提供 Node 环境管理 | `PACKAGE-STATIC` 表面不足以定 ownership | owner、scope、创建/复用/清理合同 |
| 计算环境 | GPU | 部分：Science 提供模式；主机/GPU/driver 属于外部环境 | `SCIENCE-NATIVE → SCIENCE-EXTERNAL` | `不托管`；只在未来明确支持的平台上评估兼容 | 是：不管理 GPU 主机、driver 或安全策略 | `OFFICIAL` | 第三方兼容性、sandbox 风险边界与专用主机 live |
| Skills | 官方 catalog、marketplace、remote/private sources | 是：Science/账号面 | `SCIENCE-NATIVE → OFFICIAL-ENTITLEMENT` | `原生保留` | 是：CSSwitch 不模拟官方 catalog 或 entitlement | `OFFICIAL`、`PACKAGE-STATIC` | 指定 package/account 的第三方可用性 |
| Skills | 已安装 Skill discovery 与只读投影 | 是：Science active-org Skill 目录；CSSwitch 只安全投影 | `NARROW-BRIDGE → SCIENCE-NATIVE` | `窄桥接` | 否 | `SOURCE`、`TEST` | final artifact/runtime 中目录变化与 restart 后一致性 |
| Skills | 外部 local archive / 准确公开 GitHub URL 的安装与 attach/detach | 部分：Science 拥有 Agent binding；CSSwitch 拥有外部来源 bridge | `NARROW-BRIDGE → SCIENCE-NATIVE` | `窄桥接` | 否 | `SOURCE`、`TEST`、`FIXTURE` | final artifact/runtime 的安装、卸载、attach/detach 与 restart |
| Skills | session load、trigger、domain execution | 是：Science Agent/session 与 Skill 自身工具环境 | `SCIENCE-NATIVE`；模型/tool stage 可进入 `MODEL-GATEWAY` | `原生保留`；CSSwitch 不从 attach 推断 load | 是：不实现 Agent trigger/domain executor | `PACKAGE-STATIC`、`SOURCE` 只证明前置链路 | 实际 `skill()` load、tool/poll、领域执行和 restart persistence |
| MCP | 外部 Skill 自带 internal local stdio MCP | 混合：Science loader 执行；CSSwitch 只为受管 Skill 安装 | `NARROW-BRIDGE → SCIENCE-NATIVE` | `窄桥接`，仅服务外部 Skill | 否 | `SOURCE`、`TEST`、`FIXTURE` | final artifact/runtime 的 process/tool lifecycle |
| MCP | 通用 local stdio MCP 管理 | 是：Science Settings/loader/process | `SCIENCE-NATIVE` | `原生保留` | 是：CSSwitch 不提供通用 command/args/env 管理面 | `OFFICIAL`、`PACKAGE-STATIC` | 第三方模式实际配置、启动、继承与调用 |
| MCP | custom Remote SSE / Streamable HTTP、OAuth/header | 是：Science custom-connector client；远端服务归外部 owner | `SCIENCE-NATIVE → SCIENCE-EXTERNAL`；transport 可经 HTTPS proxy/`CONNECT` | `不托管` | 是：CSSwitch 不提供通用 Remote MCP 管理面 | `OFFICIAL`、`PACKAGE-STATIC`；raw CONNECT 只属 transport | 第三方 client/auth/tool discovery/tool call |
| Connectors | Featured、Directory 与 Anthropic-hosted MCP | 是：Science/Anthropic/账号/org entitlement | `SCIENCE-NATIVE → OFFICIAL-ENTITLEMENT` | `不托管` | 是：不模拟 hosted entitlement 或 connector directory | `OFFICIAL`；Anthropic host denial 有 `SOURCE` 边界 | UI、账号 entitlement、OAuth 与用户可见 live |
| Plugins | manifest、UI、hooks、权限、启停和更新 runtime | 是：Science 上游表面 | `SCIENCE-NATIVE` | `原生保留` | 是：CSSwitch 不提供 Plugin runtime/registry；只支持可安全降解的 Skill 子集 | `OFFICIAL`、`PACKAGE-STATIC`、`SOURCE` | 完整上游合同、第三方 runtime 与各导入路径行为 |
| 检索 | Web Search | 部分：Science/Anthropic 服务与账号；外网为外部条件 | `SCIENCE-NATIVE → OFFICIAL-ENTITLEMENT` | `不托管` | 是：第三方 provider routing 不替代 Web Search | `OFFICIAL`；Anthropic 目标 transport denial 有 `SOURCE` 边界 | 用户可见结果、账号 entitlement 与 current live |
| 文献 | 开放数据库、出版商、机构代理与用户凭证访问 | 混合：Science connector/client 与具体数据服务 | `SCIENCE-NATIVE → SCIENCE-EXTERNAL`；HTTPS transport 可经 `CONNECT` | `不托管` | 是：不管理机构/付费凭证，不绕过 paywall | `OFFICIAL` | 开放、机构、付费来源分别的账号、网络和 live |
| 云数据 | S3、GCS、Azure、S3-compatible 导入 | 混合：Science client；用户/云服务拥有凭证、bucket 和费用 | `SCIENCE-NATIVE → SCIENCE-EXTERNAL`；具体 transport 按协议判断 | `不托管` | 是：CSSwitch 不管理云凭证、bucket policy 或数据 | `OFFICIAL`、`PACKAGE-STATIC` | 第三方 current live、权限、费用与数据边界 |
| Remote compute | 系统 SSH、HPC、SLURM | 混合：Science parser/record；CSSwitch opt-in bridge；用户/OpenSSH/server/scheduler 各自拥有后续层 | `NARROW-BRIDGE → SCIENCE-NATIVE → SCIENCE-EXTERNAL` | `窄桥接` preflight/sidecar/stub；不拥有 key/server/scheduler | 否 | `SOURCE`、`TEST` | parser、OpenSSH invocation、真实 server connectivity 三道动态结果 |
| Remote compute | Modal jobs | 混合：Science client；Modal 账号、预算和执行归外部服务 | `SCIENCE-NATIVE → SCIENCE-EXTERNAL`；具体 transport 按 client 判断 | `不托管` | 是：不管理真实付费任务、预算或账号 | `OFFICIAL`、`PACKAGE-STATIC` | 审批、提交、取消、费用与 current live |
| 外部推理 | BioNeMo / inference endpoints | 混合：Science client；NVIDIA/第三方 endpoint 归外部服务 | `SCIENCE-NATIVE → SCIENCE-EXTERNAL`；不进入 `MODEL-GATEWAY` 语义 | `不托管`；不与 CSSwitch provider selector 合并 | 是：不管理该 endpoint 的凭证和计费 | `OFFICIAL`、`PACKAGE-STATIC` | 第三方 endpoint 的 auth、network、billing 与 live |
| 审查 | Reviewer / Specialist | 是：Science review/specialist control plane | `SCIENCE-NATIVE → (MODEL-GATEWAY \| OFFICIAL-ENTITLEMENT)`，按实际 operation 分支 | `原生保留` | 是：CSSwitch 不提供通用 Reviewer/Specialist 设置或替代实现 | `OFFICIAL`、`PACKAGE-STATIC`、`ISOLATED-LIVE`（B-CONTEXT surface/request-shape scope，服务端结果仍 `UNVERIFIED`） | 账号/plan entitlement、第三方实际模型请求与审查结果 |
| 账号与组织 | Claude OAuth、官方模型 catalog、订阅、usage credits/spend limit、entitlement | 否：由 Anthropic 账号、组织和服务拥有 | `OFFICIAL-ENTITLEMENT` | `不托管`；第三方模式只使用本地虚拟登录 | 是：不模拟真实 OAuth、usage credits 或 entitlement | `OFFICIAL`（2026-08-14 audit）、`SOURCE` | 指定账号/组织的可用能力属于官方模式取证 |
| 账号与组织 | telemetry、analytics/Admin API、offboarding、compliance、数据驻留 | 混合：Science local data 与 Anthropic 服务端是不同数据面 | `SCIENCE-NATIVE → OFFICIAL-ENTITLEMENT` | `不托管` | 是：CSSwitch 不提供组织管理/compliance API | `OFFICIAL`、`PACKAGE-STATIC` | 本地/服务端数据边界、账号计划与 current live |
| Network | app、model、connector、sandbox、preview/voice、package、remote compute、updater 各流量面 | 混合：Science 发起流量；服务与网络 owner 各异 | 按 operation 组合全部 stage；语义链与 socket transport 分开 | `必须托管`第三方 model/Gateway 必需的受限 network policy；其他流量不合并承诺 | 部分：除第三方 model/Gateway 必需流量外均非目标 | `SOURCE`、`TEST`、`PACKAGE-STATIC`；历史 live 只按各 evidence 的 socket/operation scope | 官方 current Science 各流量面的独立 live 结果与非 Gateway transport coverage |
| Updater | Science check/apply/update/rollback | 是：Science updater 与最低支持版本 | `SCIENCE-NATIVE → SCIENCE-EXTERNAL`；候选采用另走 `CSSWITCH-RUNTIME` | `原生保留`；第三方运行固定 `--no-auto-update`，启动时只采用受校验候选；content-addressed snapshot 不等于差异 ledger | 是：CSSwitch 不提供通用 check/apply/download UI | `SOURCE`、`TEST`、`PACKAGE-STATIC`；历史 `ARTIFACT` / `ISOLATED-LIVE`（pending/adoption scope） | 当前 exact source + 官方 current Science candidate/adoption、installed identity、rollback 与 public release；CSSwitch 应用发布证据另行治理 |
| Codex | browser-only OAuth、模型目录与 Responses bridge | 否：Codex 与第三方模型服务各有 owner | `NARROW-BRIDGE → MODEL-GATEWAY`；provider 只作为 external dependency | `窄桥接`，默认关闭且不读取原生 `~/.codex` | 否 | `SOURCE`、`TEST`、`FIXTURE` | final artifact、真实账号与 provider live |
| 诊断 | Gateway/Science/upstream 状态、doctor 与 reconciliation | 否：这是 CSSwitch 运维责任 | `CSSWITCH-RUNTIME`，并按失败 stage 投影其他类别 | `必须托管` | 否 | `SOURCE`、`TEST` | final artifact、installed-live 与真实故障覆盖 |

## 决策结论

CSSwitch 第三方模式真正必须托管的只有：

1. 隔离 HOME/data-dir、runtime identity、启动/停止/恢复；
2. 本地虚拟登录、loopback Gateway、provider/profile/model routing；
3. 第三方模式所需的受限 network/route policy；
4. 与上述受管状态一致的诊断和恢复。

其中第二项不以“文本聊天成功”为完成。凡 Science 原生能力通过模型请求实现，
CSSwitch 都负有 `MODEL-GATEWAY` 协议兼容责任：支持 provider 明确具备的能力，
对不具备或尚未验证的能力给出可定位降级，不得静默改写为另一种语义。

OpenAI Chat / Responses 的图片输入、reasoning 回放、工具验证与成功停止条件，
统一见[Gateway 内容与串流边界](../architecture/gateway-provider-routing.md#openai-chat-responses-内容与串流)。

外部 Skill、系统 SSH 和 Codex 属于窄桥接：CSSwitch 只负责自己明确建立的入口，
不接管 Science Agent、OpenSSH/server 或 Codex/第三方服务的后续语义。

Project/session/artifact、权限、memory、annotations、Python/R/Conda、Skill
load/trigger、Reviewer/Specialist 等能力继续由 Science 原生管理；CSSwitch
仍须保证 `SCIENCE-NATIVE` 包络不破坏它们，并验证第三方模型路径所需的底层协议
能力。官方账号、
entitlement、catalog/hosted connectors、通用 MCP/Plugin 管理、云服务、组织管理
和真实付费计算当前不是 CSSwitch 第三方模式的托管目标。受管 Skill / MCP / Plugin
子集的[目标架构合同](../architecture/skill-mcp-plugin-control-plane.md)已经接受；inspect-only
source adapter 已存在但没有产品 caller，plan / apply、exact artifact 与 runtime evidence
仍为 `NOT-RUN`。在各组件真正形成产品可达性并取得对应证据前，不得改写表中的
production ownership、可达性或证据层。

## 维护规则

- 一项能力只在本表保存一个当前 ownership/托管结论；
- 新 Science 版本只能在日期化 audit 中记录观察，证据足够后再更新本表；
- `UNKNOWN` 不是待实现列表；先判断 owner，再决定是否属于 CSSwitch；
- `原生保留` 和 `不托管` 仍须分别标明其 stage 链，不能用 ownership 回避
  model、sandbox 或 bridge 的必要责任；
- mock、fake、fixture、source、test、package-static、artifact 与 installed-live
  必须保持分层；
- 不因能力存在于 Science UI、route 或字符串中，就承诺第三方模式 current live；
- 已接受的 Skill/MCP/Plugin 目标合同只冻结支持类型、来源、权限、生命周期、
  故障/日志与 Science ownership；inspect-only 实现、apply 能力及各证据层开始或完成推进时，
  必须分别复核目标合同与本表，未取得对应证据前不得改写为当前能力；
- 不为本表新增 handoff 流水线、探针执行、GATE-SOURCE、schema、lint 或 lifecycle
  合同。
