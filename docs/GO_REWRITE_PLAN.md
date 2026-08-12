# Discourse Go 重构计划

状态：提案（Phase 0）  
目标分支：`agent/go-rewrite-plan`  
最终目标：以 Go 完整替代 Ruby/Rails 后端，并同时支持原生服务器与 Cloudflare Workers（Go/Wasm）部署。

## 1. 不可变目标

1. 最终产物不依赖 Ruby、Rails、Bundler、Sidekiq 或 Ruby Gem。
2. 现有 PostgreSQL 仍是完整支持的主数据库；新增 Cloudflare D1（SQLite 语义）作为 Workers 数据库。
3. 现有 Redis 仍可用于原生部署；新增 Workers KV 作为 Cloudflare 部署中的缓存、会话和低一致性临时数据层。
4. Go 领域逻辑、权限规则和 API 序列化代码必须在原生与 Wasm 两种目标之间复用，不能维护两套业务实现。
5. 保留对现有 Discourse 前端 API 的兼容，迁移期间以契约测试证明兼容性。
6. 每个功能域完成 Go 替代并通过验收后，在同一个合并单元中删除对应 Rails 实现；Rails 不能成为长期回退后端。
7. 不为了兼容 Redis 而错误地把 KV 当作强一致数据库。需要原子计数、锁、队列或实时广播的功能必须使用数据库事务、Durable Objects、Queues 或其他明确适配器。

## 2. 最终架构

```text
clients / existing Ember frontend
                 |
           Discourse HTTP API
                 |
       Go application/domain layer
          /                  \
 native runtime          Workers runtime
 PostgreSQL              D1
 Redis                   KV
 filesystem/S3           R2
 background workers      Queues/Workflows
 websocket service       Durable Objects
```

### 2.1 Go 包边界

```text
cmd/discourse/             原生 HTTP 服务入口
cmd/migrate/               数据库迁移和数据导入入口
internal/domain/           用户、主题、帖子、权限等纯领域模型
internal/app/              用例和事务边界
internal/httpapi/          Discourse 兼容路由、验证和序列化
internal/ports/            数据库、缓存、队列、对象存储接口
internal/platform/native/  PostgreSQL、Redis、S3/文件系统适配器
internal/platform/worker/  D1、KV、R2、Queues/DO 的 js/wasm 适配器
worker/                    极薄的 Workers JavaScript 启动和绑定桥
migrations/postgres/       PostgreSQL 迁移
migrations/d1/             D1/SQLite 迁移
contracts/                 从现有 Discourse 固化的 API 契约样例
web/                       保留并逐步适配现有前端
```

平台包只负责 I/O。权限、审核、信任等级、主题状态、帖子规则等不得放进 JavaScript 桥接层或数据库适配器。

## 3. 数据与缓存兼容矩阵

| 能力 | 原生部署 | Workers 部署 | 一致性要求 |
| --- | --- | --- | --- |
| 关系数据 | PostgreSQL 15+ | D1 | 强一致写；显式事务 |
| 查询缓存 | Redis 7+ | KV | 可最终一致；必须允许缓存失效/漏读 |
| 登录会话 | Redis 或数据库 | KV + 数据库校验 | 撤销信息以数据库为准 |
| 限流计数 | Redis | Durable Object | 强一致计数，不直接使用 KV |
| 后台任务 | Go worker/Redis 队列 | Queues/Workflows | 至少一次；处理器幂等 |
| 上传 | 文件系统或 S3 | R2 | 对象键写入数据库 |
| 实时消息 | Go WebSocket 服务 | Durable Object | 连接状态不写 KV |
| 搜索 | PostgreSQL 搜索/外部引擎 | 初期 D1 FTS5，后续可插拔 | 索引最终一致 |

> KV 仅替代 Redis 的缓存类用途，不声称覆盖 Redis 的 Lua、Pub/Sub、Stream、锁和强一致原子操作。

## 4. 迁移方法

每个阶段都使用相同的替换流程：

1. 从 Rails controller、model、serializer、guardian 和测试中提取 API/权限契约。
2. 在 `contracts/` 固化请求、响应、状态码和权限矩阵。
3. 在 Go 中实现领域规则与存储端口。
4. 同时实现 PostgreSQL/Redis 和 D1/KV 适配器。
5. 在两种数据库上运行相同的领域与 API 测试。
6. 对真实 Discourse PostgreSQL 备份执行迁移演练，并生成差异报告。
7. 删除本阶段已替代的 Rails 路由、controller、model、job 和测试。
8. 更新兼容矩阵；只有通过阶段门槛后才合并。

任何阶段都不得通过“未实现时代理回 Rails”来过验收。

## 5. 分阶段路线图

### Phase 0：基线与契约冻结

交付物：

- 统计 Rails 路由、模型、后台任务、序列化器、插件扩展点和站点设置。
- 生成首批 API 契约夹具和数据库模式快照。
- 建立 Go、PostgreSQL、D1、Redis、KV 的 CI 测试矩阵。
- 建立功能兼容清单，状态仅允许 `unmapped`、`contracted`、`implemented`、`verified`、`ruby-removed`。
- 确定 URL、JSON 字段、错误码和分页策略的兼容范围。

退出条件：范围可计数；核心契约可自动运行；没有凭记忆猜测的 API。

### Phase 1：运行时骨架与身份系统

范围：

- 配置加载、结构化日志、健康检查和优雅关闭。
- 用户、邮箱、密码哈希、API key、登录、登出、会话撤销。
- 用户组、基础角色和 Guardian 权限核心。
- PostgreSQL/Redis 原生启动；Go/Wasm Workers 启动；D1/KV 绑定。
- 双数据库迁移框架与相同的 repository 契约测试。

退出条件：原生和 Workers 均能注册、登录、读取当前用户并撤销会话；对应 Rails 身份路由被删除。

### Phase 2：论坛核心

范围：

- 分类、标签、主题、帖子、草稿、编辑历史和删除/恢复。
- 主题列表、时间线、分页、置顶、关闭、归档和可见性。
- Markdown 烹制接口与基础 onebox 安全边界。
- 点赞、书签和阅读状态。

退出条件：可用现有前端完成浏览、发主题、回复、编辑、删除和恢复；两种数据库契约一致；对应 Rails 实现被删除。

### Phase 3：审核、信任与管理

范围：

- 举报、审核队列、封禁/禁言、慢速模式和自动关闭。
- 信任等级计算、徽章基础、用户状态和管理日志。
- 站点设置、分类权限、组权限及管理员 API。
- 限流端口：Redis 与 Durable Object 分别实现。

退出条件：权限矩阵和审核状态机与冻结契约一致；安全相关差异为零。

### Phase 4：通知、实时与后台任务

范围：

- 通知、提及、引用、关注、摘要和邮件任务。
- 原生后台 worker 与 Cloudflare Queues/Workflows。
- MessageBus 兼容层：原生 WebSocket 与 Durable Objects。
- 幂等键、重试、死信和任务可观测性。

退出条件：任务可安全重放；通知不重不漏达到契约要求；不依赖 Sidekiq/Redis PubSub。

### Phase 5：搜索、上传与内容处理

范围：

- PostgreSQL 搜索和 D1 FTS5；保留外部搜索适配端口。
- 文件系统/S3 与 R2 上传；缩略图和安全扫描任务。
- 链接、预览、站点地图、RSS、备份与恢复。

退出条件：搜索排序有固定评测集；上传权限和对象生命周期通过集成测试。

### Phase 6：前端兼容与用户体验收口

范围：

- 完成现有 Ember 前端依赖的 API。
- 替换 Rails 资产辅助、服务端预加载和引导 JSON。
- 移除前端对 Rails-only endpoint、CSRF helper 和模板注入的依赖。
- 完成浏览器端关键流程测试与无障碍基线。

退出条件：普通用户、版主和管理员关键流程在原生与 Workers 环境全部通过。

### Phase 7：插件、主题与迁移工具

范围：

- 定义 Go 插件能力边界；不能直接运行 Ruby 插件。
- 保持客户端主题组件兼容；为服务端插件提供清晰迁移 SDK/API。
- 实现从官方 Discourse PostgreSQL/上传目录导入的可恢复迁移工具。
- 输出逐插件兼容报告，不伪装成 100% Ruby 插件兼容。

退出条件：迁移工具能断点续跑、校验行数/哈希/外键；插件不兼容项有明确列表。

### Phase 8：Ruby 清零与发布加固

范围：

- 删除剩余 `.rb`、Gemfile、Bundler、Rails、Sidekiq 和 Ruby CI/容器配置。
- 删除临时契约提取脚本与过渡开关。
- 原生负载测试、Wasm 大小/启动时间测试、D1/KV 配额测试。
- 安全审计、依赖清单、升级/回滚手册和正式发行包。

退出条件：仓库运行与测试不需要 Ruby；Ruby 源文件计数为零；两种部署目标均有可复现发布产物。

## 6. API 兼容优先级

按现有前端和常见客户端的依赖程度迁移：

1. `/site.json`、`/session/current.json`、登录和引导数据。
2. 分类、主题列表、主题详情、发主题、回复与编辑。
3. 用户、组、通知、草稿、书签、点赞和搜索。
4. 版主、管理员、备份、邮件和站点设置。
5. 低使用率 API 与明确记录的插件扩展端点。

兼容意味着契约明确，不意味着继续暴露已知不安全行为。安全修复产生的差异必须写入变更日志和契约例外清单。

## 7. 数据迁移原则

- PostgreSQL 原有选择被保留，但 Go 版使用独立、版本化的迁移，不运行 Rails migration。
- 第一版 PostgreSQL 适配优先读取现有 Discourse 表结构，逐阶段迁移到 Go 拥有的稳定模式。
- D1 使用独立 SQL migration；所有共享查询必须经过方言层，不在业务代码中散布 PostgreSQL/SQLite 判断。
- ID、slug、时间、软删除、唯一约束和外键必须有跨数据库契约测试。
- 大型迁移按批次执行并带游标、校验和、进度记录与幂等重试。
- 切换前必须生成用户、主题、帖子、上传和权限关系的计数/抽样哈希报告。

## 8. Workers 实现约束

- Go 编译为 Wasm；JavaScript 仅负责启动 Go runtime、把 `Request`/bindings 传入 Go，并把 Go 响应转回 `Response`。
- D1、KV、R2、Queues 和 Durable Objects 使用运行时绑定，不从 Worker 内调用 Cloudflare REST API。
- D1 查询使用绑定参数和批处理；禁止字符串拼接 SQL。
- KV 读必须容忍旧值和空值；鉴权撤销、权限和计数不能只依赖 KV。
- Wasm 包大小、启动时间、CPU 时间和内存进入 CI 预算；超预算即失败。
- 后台 Promise 必须等待或交给 `waitUntil`；请求级状态不得存入全局可变变量。

## 9. 测试与合并门槛

每个替换阶段至少需要：

- Go 单元测试和 race test（原生目标）。
- PostgreSQL repository 集成测试。
- D1/Miniflare repository 集成测试。
- Redis 与 KV 缓存语义测试。
- API golden/contract tests。
- 数据迁移演练和回滚测试。
- Workers `wrangler deploy --dry-run` 与 Wasm 大小预算。
- 对应 Ruby 文件删除清单；未删除必须有下一阶段归属和原因。

不允许用跳过测试、空实现、固定假数据或 Rails 代理满足阶段退出条件。

## 10. 分支与提交策略

- 总路线图：`agent/go-rewrite-plan`。
- 实现分支：按阶段使用 `agent/go-phase-N-<scope>`。
- 每个 PR 只替换一个可验收功能域，并在同一 PR 删除该域 Ruby 实现。
- 主分支始终保持可构建；尚未迁移的域在原系统中继续存在，已迁移的域不再保留 Rails 回退。
- Phase 8 合并后打第一个 Go-only 预发布版本，再开始稳定性发布周期。

## 11. 当前明确的不兼容项

- 任意 Ruby 服务端插件不能直接在 Go/Wasm 中运行，需要迁移或隔离服务。
- Workers KV 不能完整替代 Redis 的强一致和消息能力。
- D1 是 SQLite 语义，不可能逐条照搬所有 PostgreSQL SQL、扩展和全文搜索行为。
- Workers 不提供传统常驻进程、任意文件系统和 Sidekiq 运行模型。

这些限制必须通过明确适配和兼容报告解决，不能用同名接口掩盖不同语义。
