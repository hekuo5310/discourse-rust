# Forum Engine

[English](README_en.md)

[![一键部署到 Cloudflare](https://deploy.workers.cloudflare.com/button)](https://deploy.workers.cloudflare.com/?url=https://github.com/hekuo5310/discourse-rust)

一个以 Rust 为主要实现语言、通过 clean-room 方式独立开发的社区讨论平台。

本仓库当前代码树不包含或改写此前项目的源代码、测试、素材或文案。临时名称
“Forum Engine”保持中立，后续可以替换。旧项目提交仍保留在 Git 历史中。

本项目采用 [Apache2.0](LICENSE) 许可证。

## 已实现

- 所有运行时共享的 Rust 领域核心。
- 编译为 WebAssembly 的 Cloudflare Workers 部署方式。
- 使用 D1 存储用户、会话、分类、主题和回复。
- 使用 KV 提供会话缓存提示，同时由 D1 负责最终的会话撤销验证。
- 注册、登录、退出、当前用户、分类、主题和回复 API。
- 使用 PostgreSQL 和 Redis 的原生 Rust HTTP 运行时。
- 内嵌 PostgreSQL 迁移及基于容器的本地运行环境。
- 面向可选扩展模块的 WIT 边界；扩展可以使用 Rust、Go、C、C++、C#、
  Python 或其他能够编译到 WebAssembly Component Model 的语言。

Rust 始终是主要实现语言。只有在确有收益时，其他语言才通过带版本的组件边界接入。

## 一键部署到 Cloudflare Workers

点击上方按钮后，Cloudflare 会复制本仓库、根据 `wrangler.jsonc` 自动创建 D1
数据库和 KV 命名空间、执行 D1 迁移、构建 Rust WebAssembly Worker，并配置
Workers Builds 以便后续提交自动部署。

一键部署使用 Workers + D1 + KV。原有 PostgreSQL + Redis 部署选项仍由下方的
原生运行时提供。

## Workers 本地开发

1. 安装 JavaScript 工具：

   ```sh
   npm install
   ```

2. 应用本地 D1 迁移：

   ```sh
   npm run migrate:local
   ```

3. 启动 Worker：

   ```sh
   npm run dev
   ```

API 使用 `Authorization: Bearer <token>`。登录和注册响应只返回一次明文令牌，
数据库中仅存储其 SHA-256 摘要。

## 原生运行时快速开始

原生运行时保留 PostgreSQL 和 Redis 部署选项。使用以下命令启动完整环境：

```sh
docker compose up --build
```

API 随后可通过 `http://localhost:3000` 访问。在容器外运行二进制文件时，需要设置
`DATABASE_URL`、`REDIS_URL`，并可选设置 `LISTEN_ADDR`。PostgreSQL 是会话的
权威数据源，Redis 仅保存可丢弃的缓存提示。服务启动时会自动执行迁移。

## API

| 方法 | 路径 | 身份要求 |
| --- | --- | --- |
| GET | `/api/v1/health` | 无 |
| POST | `/api/v1/auth/register` | 无 |
| POST | `/api/v1/auth/login` | 无 |
| POST | `/api/v1/auth/logout` | 已登录 |
| GET | `/api/v1/me` | 已登录 |
| GET | `/api/v1/categories` | 无 |
| POST | `/api/v1/categories` | 管理员 |
| GET | `/api/v1/topics` | 无 |
| POST | `/api/v1/topics` | 已登录 |
| GET | `/api/v1/topics/:id` | 无 |
| POST | `/api/v1/topics/:id/posts` | 已登录 |

Workers/D1/KV 与原生 PostgreSQL/Redis 运行时提供相同的 API 契约。剩余的
clean-room 重构阶段请参阅 [`docs/ROADMAP.md`](docs/ROADMAP.md)。

## 所有权与贡献

项目暂不接收外部代码贡献，以免在建立允许未来重新许可的贡献者协议之前引入混合
版权。欢迎提交描述功能行为的 Issue，但不得提交从其他论坛实现复制、翻译或机械改写
而来的补丁。
