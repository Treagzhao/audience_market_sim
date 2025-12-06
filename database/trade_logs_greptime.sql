-- GreptimeDB建表语句 for trade_logs
CREATE TABLE trade_logs (
    -- 时间索引字段
    `timestamp` timestamp NOT NULL,
    
    -- 标签字段（用于分组、过滤的高频查询字段）
    round BIGINT,                    -- 模拟轮次
    trade_id BIGINT,                 -- 交易ID
    task_id STRING INVERTED INDEX,                  -- 任务ID
    agent_id BIGINT INVERTED INDEX,                 -- 主体ID
    agent_name STRING,               -- 主体名称
    factory_id BIGINT INVERTED INDEX,               -- 工厂ID
    factory_name STRING,             -- 工厂名称
    product_id BIGINT INVERTED INDEX,               -- 产品ID
    product_name STRING,             -- 产品名称
    trade_result STRING,             -- 交易结果（Success/Failed/NotMatched/NotYet）
    interval_relation STRING,        -- 区间关系（Overlapping/AgentBelowFactory/AgentAboveFactory）
    
    -- 字段（数值型数据，用于聚合分析）
    agent_cash DOUBLE NOT NULL,      -- 主体现金
    price DOUBLE,                    -- 成交价格（可选）
    factory_supply_range_lower DOUBLE NOT NULL, -- 工厂供应范围下限
    factory_supply_range_upper DOUBLE NOT NULL, -- 工厂供应范围上限
    factory_stock INT NOT NULL,      -- 工厂库存
    agent_pref_original_price DOUBLE, -- 主体偏好原始价格（可选）
    agent_pref_original_elastic DOUBLE, -- 主体偏好原始弹性（可选）
    agent_pref_current_price DOUBLE, -- 主体偏好当前价格（可选）
    agent_pref_current_range_lower DOUBLE, -- 主体偏好当前范围下限（可选）
    agent_pref_current_range_upper DOUBLE, -- 主体偏好当前范围上限（可选）
    
    -- 指定时间索引
    TIME INDEX (`timestamp`),
    
    -- 设置标签（提高查询性能）
    PRIMARY KEY (task_id, agent_id, factory_id, product_id, trade_id)
);

-- 说明：
-- 1. timestamp作为时间索引，用于时序查询和分析
-- 2. 标签字段选择了高频查询的维度字段，优化过滤和分组性能
-- 3. 数值型字段支持聚合分析，如平均成交价格、平均主体现金等
-- 4. 主键设计考虑了数据分布和查询模式，确保唯一性
-- 5. 支持可选字段（使用DOUBLE类型，NULL表示未设置）
-- 6. GreptimeDB自动分区和压缩，适合存储大量交易日志

-- 示例插入语句
-- INSERT INTO trade_logs VALUES (
--     1630000000000, -- timestamp
--     100,           -- round
--     1,             -- trade_id
--     'task_123',    -- task_id
--     1001,          -- agent_id
--     'Consumer_1001', -- agent_name
--     5000.0,        -- agent_cash
--     1,             -- factory_id
--     'factory_1',   -- factory_name
--     101,           -- product_id
--     'product_1',   -- product_name
--     'Success',     -- trade_result
--     'Overlapping', -- interval_relation
--     150.5,         -- price
--     100.0,         -- factory_supply_range_lower
--     200.0,         -- factory_supply_range_upper
--     9,             -- factory_stock
--     140.0,         -- agent_pref_original_price
--     0.5,           -- agent_pref_original_elastic
--     150.5,         -- agent_pref_current_price
--     120.0,         -- agent_pref_current_range_lower
--     180.0          -- agent_pref_current_range_upper
-- );

-- 示例查询
-- 查询特定任务的交易统计
-- SELECT 
--     round,
--     COUNT(*) as trade_count,
--     AVG(price) as avg_price,
--     SUM(CASE WHEN trade_result = 'Success' THEN 1 ELSE 0 END) as success_count,
--     SUM(CASE WHEN trade_result = 'Failed' THEN 1 ELSE 0 END) as failed_count
-- FROM trade_logs
-- WHERE task_id = 'task_123'
-- GROUP BY round
-- ORDER BY round;

-- 查询特定产品的交易情况
-- SELECT 
--     factory_id,
--     factory_name,
--     COUNT(*) as trade_count,
--     AVG(price) as avg_price,
--     AVG(agent_cash) as avg_agent_cash
-- FROM trade_logs
-- WHERE product_id = 101 AND trade_result = 'Success'
-- GROUP BY factory_id, factory_name;
