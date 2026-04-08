-- GreptimeDB建表语句 for agent_range_adjustment_logs
CREATE TABLE agent_range_adjustment_logs (
    -- 时间索引字段
    `timestamp` TIMESTAMP NOT NULL,
    
    -- 标签字段（用于分组、过滤的高频查询字段）
    round BIGINT,                    -- 模拟轮次
    task_id STRING INVERTED INDEX,                  -- 任务ID
    agent_id BIGINT INVERTED INDEX,                 -- 主体ID
    agent_name STRING,               -- 主体名称
    product_id BIGINT INVERTED INDEX,               -- 产品ID
    product_category STRING INVERTED INDEX,         -- 产品类别
    adjustment_type STRING,          -- 调整类型：trade_success 或 trade_failed
    
    -- 字段（数值型数据，用于聚合分析）
    old_range_lower DOUBLE NOT NULL, -- 调整前的范围下限
    old_range_upper DOUBLE NOT NULL, -- 调整前的范围上限
    new_range_lower DOUBLE NOT NULL, -- 调整后的范围下限
    new_range_upper DOUBLE NOT NULL, -- 调整后的范围上限
    lower_change DOUBLE NOT NULL,    -- 下限变化量
    upper_change DOUBLE NOT NULL,    -- 上限变化量
    min_change_ratio DOUBLE NOT NULL, -- 下限变化比例（百分比）
    max_change_ratio DOUBLE NOT NULL, -- 上限变化比例（百分比）
    center DOUBLE NOT NULL,          -- 调整中心
    price DOUBLE,                    -- 成交价格（仅交易成功时有值）
    
    -- 指定时间索引
    TIME INDEX (`timestamp`),
    
    -- 设置标签（提高查询性能）
    PRIMARY KEY (task_id, agent_id, product_id, round)
);

-- 说明：
-- 1. timestamp作为时间索引，用于时序查询和分析
-- 2. 标签字段选择了高频查询的维度字段，优化过滤和分组性能
-- 3. 数值型字段支持聚合分析，如平均调整幅度、平均变化比例等
-- 4. 主键设计考虑了数据分布和查询模式，确保唯一性
-- 5. 支持可选字段（使用DOUBLE类型，NULL表示未设置）
-- 6. GreptimeDB自动分区和压缩，适合存储大量时序数据

-- 示例插入语句
-- INSERT INTO agent_range_adjustment_logs VALUES (
--     1630000000000, -- timestamp
--     100,           -- round
--     'task_123',    -- task_id
--     1001,          -- agent_id
--     'Consumer_1001', -- agent_name
--     101,           -- product_id
--     'trade_success', -- adjustment_type
--     120.0,         -- old_range_lower
--     180.0,         -- old_range_upper
--     140.0,         -- new_range_lower
--     160.0,         -- new_range_upper
--     20.0,          -- lower_change
--     -20.0,         -- upper_change
--     16.67,         -- min_change_ratio
--     -11.11,        -- max_change_ratio
--     150.5,         -- center
--     150.5          -- price
-- );

-- 示例查询
-- 查询特定任务的主体范围调整趋势
-- SELECT 
--     round,
--     AVG(new_range_lower) as avg_new_lower,
--     AVG(new_range_upper) as avg_new_upper,
--     COUNT(*) as adjustment_count
-- FROM agent_range_adjustment_logs
-- WHERE task_id = 'task_123' AND adjustment_type = 'trade_success'
-- GROUP BY round
-- ORDER BY round;

-- 查询特定产品的主体调整情况
-- SELECT 
--     agent_id,
--     agent_name,
--     COUNT(*) as adjustment_count,
--     AVG(min_change_ratio) as avg_min_change,
--     AVG(max_change_ratio) as avg_max_change
-- FROM agent_range_adjustment_logs
-- WHERE product_id = 101
-- GROUP BY agent_id, agent_name;
