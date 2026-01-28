# 按照技术层划分需要关注一下内容
- 业务逻辑层：例如model部分
- 工具层，例如正态分布结构体、数据工具类
- 基础设施层：包括数据库、缓存、消息队列等


#按照技术层划分的合理设计
graph TD
B[业务逻辑层] --> T[工具层]
B --> I[基础设施层]

    style B fill:#e1f5fe
    style T fill:#f3e5f5
    style I fill:#e8f5e8