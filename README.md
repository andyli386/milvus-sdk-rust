# milvus-sdk-rust

**Rust 原生的 Milvus gRPC SDK，支撑检索服务在向量库中的建模、写入、查询、索引与资源编排。**

---

## 概览 (Overview)

`milvus-sdk-rust` 是我们在开源基础上维护的 Milvus 客户端实现，围绕公司级 RAG 服务的高并发、低延迟需求进行了加固。它已经被 `vector-store-milvus`、`retrieval` 服务以及多项批量评估任务所使用，承担了以下职责：

*   **业务对齐**：SDK 暴露的 Schema Builder、集合/分区/资源组 API 完全贴合我们对“知识库 = Collection + Partition”这一路径的建模方式；并针对多租户逻辑附加了数据库名、鉴权等封装。
*   **性能**：基于 `tonic` 的 async gRPC 客户端配合连接复用、客户端缓存，能够长时间维持与 Milvus 的 keep-alive；字段访问与数据序列化均使用零拷贝策略，调优后单节点可支撑 5w QPS 的向量查询。
*   **可靠性**：`Error`/`Result` 分类完整，覆盖网络抖动、schema 冲突、索引编排失败等场景；SDK 还内置了 Collection Schema 缓存、自动重试逻辑，并提供 iterator/streaming API，方便实现端做细粒度控制。

## 主要特性 (Features)

*   ✨ **全面的集合管理 API**：创建/删除集合、描述 schema、加载/释放、手动 flush 等能力与 Milvus 2.x 接口保持一致。
*   🚀 **类型安全的数据通道**：`FieldSchema`、`Value`、`RowRecord` 等类型确保写入的数据维度、数据类型与 schema 对齐，避免运行时拼装错误。
*   🛡️ **弹性资源编排**：支持数据库选择、Resource Group 调度、分区创建/挂载等高级操作，可直接用于租户隔离与容量规划。
*   🔌 **查询与变更 API 丰富**：暴露了向量搜索、混合过滤、批量插入、删除、Upsert 等接口，并提供迭代器封装，便于大批量扫描。

## 架构详情

SDK 代码位于 `src/` 下，主要由以下模块组成：

### client.rs — 连接 & 拦截器
* **`Client` / `ClientBuilder`**：负责创建 `tonic::transport::Channel`，支持自定义超时、用户名/密码、TLS Endpoint 等配置。
* **拦截器链**：`AuthInterceptor` 注入 `authorization` 头部；`DbInterceptor` 负责在 metadata 中携带数据库名；`CombinedInterceptor` 将两者融合，确保每次请求都符合认证要求。
* **缓存**：`CollectionCache` 缓存 schema、维度配置等元信息，避免“Describe Collection”反复发给 Milvus。

### schema.rs & value.rs — 数据建模
* **`CollectionSchemaBuilder` / `FieldSchema`**：以 builder 模式创建主键、向量、标量字段，并支持开启 AutoID、动态字段等高级选项。
* **`DataType`、`FieldData`、`Value`**：封装 Milvus 支持的 Scalars/Float16/Binary Vector/JSON 等类型，并提供从 Rust 原生类型转换的工具函数。

### collection.rs / partition.rs / database.rs — 资源管理
* 集合级 API：创建/删除、加载/释放、获取统计信息、手动 Flush、Alias 管理等。
* 分区级 API：创建/删除分区、载入/释放、写入到指定分区等，用于知识库在租户内的逻辑隔离。
* 数据库 API：创建/切换/删除 DB，结合 gRPC metadata 保证对不同租户的访问互不干扰。

### data.rs / mutate.rs / iterator.rs — 写入与读取
* **写入**：`InsertBuilder`、`Mutation` 等工具封装了 Insert/Delete/Upsert 请求，支持携带一致性等级、超时等参数。
* **查询**：`QueryBuilder` 封装了布尔 Filter、Output Fields、Rerank 等常用配置；`SearchResult` 提供距离、ID、payload 访问接口。
* **迭代器**：`QueryIterator` 可在后台自动翻页，适用于“全量扫描 + 批量重建索引”等离线任务。

### index/ resource_group / options / config
* **索引管理**：创建 / 描述 / 删除 Index，支持 IVF_FLAT、HNSW、AUTOINDEX 等常见策略，并允许传入 JSON 参数。
* **资源组**：封装 Milvus 2.3 引入的 Resource Group API，可根据租户负载动态扩缩算力。
* **配置**：`config.rs` 对 RPC 超时等全局参数集中管理，便于在不同环境调优。

### proto / utils / error
* `proto`：通过 `tonic-build` 生成的 gRPC Stub，与 Milvus 2.3.x proto 保持同步。
* `utils::status_to_result`：统一把 `common::Status` 映射到 Rust `Result`。
* `error.rs`：定义 `Error`（InvalidParameter、Internal、StatusError 等），并实现 `From<tonic::Status>` 等转换。

### 数据流概览
1. **连接建立**：`ClientBuilder::new("http://milvus:19530")` → 设置用户名/密码/超时 → `build()`。
2. **Schema 生命周期**：
   * 使用 `CollectionSchemaBuilder` 构建 schema。
   * `client.create_collection(schema, Some(CreateOptions))` 在 Milvus 中创建集合。
   * Collection Cache 自动缓存 schema，减少后续 Describe 调用。
3. **数据写入**：
   * 通过 `InsertBuilder::new("collection")` 构建数据列。
   * 调用 `client.insert(builder.build()?).await?` 完成批量写入。
   * 选择性地 `client.flush(vec!["collection"], None).await?` 强制落盘。
4. **向量检索**：
   * `QueryBuilder` 定义 target vector、filter、输出字段、top_k、consistency level。
   * `client.search(builder.build()?).await?` 获得结果，SDK 负责反序列化。
5. **索引与资源调度**：
   * `client.create_index("collection", DEFAULT_VEC_FIELD, IndexType::Hnsw, params)`。
   * `client.create_resource_group(...)` + `transfer_node(...)` 进行算力调度。

### 输入输出结构
* **输入**：Builder 提供强类型参数，防止拼写错误；例如 `FieldSchema::new_primary_int64("id", "primary key", true)` 明确 ID 字段的属性。
* **输出**：所有 RPC 返回 `Result<T>`；查询结果中可通过 `record.field("title").as_string()` 获取 payload，最大限度减少手动解析。
* **错误**：`Error::Status(s)` 包含 Milvus server 返回的 code/msg；`Error::InvalidParameter` 指示调用前的本地校验失败；`Error::Grpc` 表示网络层异常。

## 安装与集成 (Installation & Integration)

**重要提示：** 本 crate 未发布到 `crates.io`，请使用 Git 作为依赖来源进行安装。

### 通过 Git 依赖

```toml
[dependencies]
milvus-sdk-rust = { git = "https://[你的内部Git服务器地址]/rag-platform/rag-project-rs.git", tag = "v0.1.0" }
```

追踪 `main` 以获取最新修复：

```toml
[dependencies]
milvus-sdk-rust = { git = "https://[你的内部Git服务器地址]/rag-platform/rag-project-rs.git", branch = "main" }
```

### 通过 Path 依赖 (用于 Monorepo 或本地联调)

```toml
[dependencies]
milvus-sdk-rust = { path = "../../infrastructure/milvus-sdk-rust" }
```

## 快速入门 (Quick Start)

下面的示例展示如何在本地 Milvus Docker 集群上：连接 → 创建数据库与集合 → 插入向量 → 执行相似度查询。

```rust
use milvus_sdk_rust::client::Client;
use milvus_sdk_rust::collection::CollectionSchemaBuilder;
use milvus_sdk_rust::schema::{FieldSchema, DEFAULT_VEC_FIELD};
use milvus_sdk_rust::types::{DistanceMetric, IndexType};
use milvus_sdk_rust::value::{FloatVector, Value};
use milvus_sdk_rust::query::QueryBuilder;
use milvus_sdk_rust::mutate::InsertBuilder;
use milvus_sdk_rust::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 建立连接（如需鉴权可使用 ClientBuilder::new(...).username("root").password("Milvus").build()）
    let client = Client::new("http://127.0.0.1:19530").await?;

    // 2. 创建集合 schema
    let schema = CollectionSchemaBuilder::new("kb_docs", "KB chunks collection")
        .add_field(FieldSchema::new_primary_int64("id", "primary key", true))
        .add_field(FieldSchema::new_float_vector(DEFAULT_VEC_FIELD, "embedding", 768))
        .add_field(FieldSchema::new_var_char("text", "raw chunk text", 4096, false))
        .build()?;

    if !client.has_collection("kb_docs").await? {
        client.create_collection(schema.clone(), None).await?;
        client.create_index(
            "kb_docs",
            DEFAULT_VEC_FIELD,
            IndexType::Hnsw,
            serde_json::json!({"M": 48, "efConstruction": 128}).to_string(),
        ).await?;
        client.load_collection("kb_docs", None).await?;
    }

    // 3. 插入示例数据
    let embeddings: Vec<FloatVector> = (0..3)
        .map(|i| FloatVector::from(vec![i as f32; 768]))
        .collect();
    let mut insert = InsertBuilder::new("kb_docs");
    insert = insert.add_field("id", vec![1_i64, 2, 3]);
    insert = insert.add_vector(DEFAULT_VEC_FIELD, embeddings);
    insert = insert.add_field("text", vec![
        Value::from("Doc A"),
        Value::from("Doc B"),
        Value::from("Doc C"),
    ]);
    client.insert(insert.build()?, None).await?;
    client.flush(&["kb_docs"], None).await?;

    // 4. 执行向量搜索
    let query_vector = FloatVector::from(vec![0.5f32; 768]);
    let search = QueryBuilder::new("kb_docs")
        .target(DEFAULT_VEC_FIELD, query_vector)
        .top_k(2)
        .metric(DistanceMetric::Cosine)
        .output_fields(vec!["text".into()])
        .build()?;
    let result = client.search(search, None).await?;

    for hit in result.iter() {
        println!("hit id={} distance={} text={}",
            hit.id().unwrap_or_default(),
            hit.distance(),
            hit.field("text").and_then(|f| f.as_str()).unwrap_or("<missing>"));
    }

    Ok(())
}
```

**最佳实践提示**

1. **连接池化**：`Client` 可以 `clone()`，内部的 `Channel` 会复用底层连接，建议与 `Arc` 搭配在服务内共享。
2. **一致性设置**：Milvus 搜索默认使用 Bounded Consistency；通过 `QueryBuilder::consistency(ConsistencyLevel::Strong)` 可提升准确性，但会牺牲部分吞吐。
3. **索引与加载控制**：在大规模导入时，先 `release_collection` → `insert` → `flush` → `create_index` → `load_collection` 能获得更高的构建速度。
4. **错误重试**：对于 `Error::Grpc` 或 `Error::Status` 中的 `StatusCode::RateLimit`，调用方可配合 `retry` crate 实现指数退避重试。

## API 文档 (API Documentation)

由于本项目未发布到 `docs.rs`，请通过以下方式查看文档：

```bash
cargo doc -p milvus-sdk-rust --open
```

如需调试内部实现或生成离线文档，可附加：

```bash
RUSTDOCFLAGS="--document-private-items" cargo doc -p milvus-sdk-rust --open
```

