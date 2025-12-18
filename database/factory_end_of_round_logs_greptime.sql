-- GreptimeDB建表语句 for factory_end_of_round_logs
CREATE TABLE factory_end_of_round_logs (
    -- 时间索引字段
    `timestamp` timestamp NOT NULL,
    
    -- 标签字段（用于分组、过滤的高频查询字段）
    round BIGINT,                    -- 模拟轮次
    task_id STRING INVERTED INDEX,                  -- 任务ID
    factory_id BIGINT INVERTED INDEX,               -- 工厂ID
    factory_name STRING,             -- 工厂名称
    product_id BIGINT INVERTED INDEX,               -- 产品ID
    product_category STRING INVERTED INDEX,         -- 产品类别
    
    -- 字段（数值型数据，用于聚合分析）
    cash DOUBLE NOT NULL,            -- 工厂现金
    initial_stock INT NOT NULL,      -- 初始产量
    remaining_stock INT NOT NULL,    -- 剩余库存
    supply_range_lower DOUBLE NOT NULL, -- 供应范围下限
    supply_range_upper DOUBLE NOT NULL, -- 供应范围上限
    
    -- 指定时间索引
    TIME INDEX (`timestamp`),
    
    -- 设置标签（提高查询性能）
    PRIMARY KEY (task_id, factory_id, product_id, round)
);

-- 说明：
-- 1. timestamp作为时间索引，用于时序查询和分析
-- 2. 标签字段选择了高频查询的维度字段，优化过滤和分组性能
-- 3. 数值型字段支持聚合分析，如平均工厂现金、平均剩余库存等
-- 4. 主键设计考虑了数据分布和查询模式，确保唯一性
-- 5. GreptimeDB自动分区和压缩，适合存储大量轮次结束日志

-- 示例插入语句
-- INSERT INTO factory_end_of_round_logs VALUES (
--     1630000000000, -- timestamp
--     100,           -- round
--     'task_123',    -- task_id
--     1,             -- factory_id
--     'factory_1',   -- factory_name
--     101,           -- product_id
--     1500.5,        -- cash
--     7,             -- remaining_stock
--     100.0,         -- supply_range_lower
--     200.0          -- supply_range_upper
-- );

-- 示例查询
-- 查询特定任务的工厂现金变化趋势
-- SELECT 
--     round,
--     factory_id,
--     factory_name,
--     AVG(cash) as avg_cash,
--     AVG(remaining_stock) as avg_remaining_stock
-- FROM factory_end_of_round_logs
-- WHERE task_id = 'task_123'
-- GROUP BY round, factory_id, factory_name
-- ORDER BY round;

-- 查询特定产品的工厂库存变化
-- SELECT 
--     round,
--     factory_id,
--     factory_name,
--     remaining_stock
-- FROM factory_end_of_round_logs
-- WHERE product_id = 101 AND factory_id = 1
-- ORDER BY round;
