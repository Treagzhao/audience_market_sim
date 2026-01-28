# 按照领域层划分需要关注以下内容
- 市场 Market
- 生产者 Factory
- 商品 Product
- 消费者 Agent


# 按照领域层划分标准
flowchart TD
M[市场] --> C[消费者]
M --> P[生产者]
M --> G[商品]

    P --> G
    C --> G
