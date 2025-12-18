-- GreptimeDB建表语句 for factory_range_optimization_logs
CREATE TABLE factory_range_optimization_logs (
    -- 时间索引字段
    `timestamp` TIMESTAMP NOT NULL,
    
    -- 标签字段（用于分组、过滤的高频查询字段）
    round BIGINT,                    -- 模拟轮次
    task_id STRING INVERTED INDEX,                  -- 任务ID
    factory_id BIGINT INVERTED INDEX,               -- 工厂ID
    factory_name STRING,             -- 工厂名称
    product_id BIGINT INVERTED INDEX,               -- 产品ID
    product_category STRING INVERTED INDEX,         -- 产品类别
    trade_result STRING,             -- 交易结果（Success/Failed）
    
    -- 字段（数值型数据，用于聚合分析）
    old_range_lower DOUBLE NOT NULL, -- 优化前的范围下限
    old_range_upper DOUBLE NOT NULL, -- 优化前的范围上限
    new_range_lower DOUBLE NOT NULL, -- 优化后的范围下限
    new_range_upper DOUBLE NOT NULL, -- 优化后的范围上限
    lower_change DOUBLE NOT NULL,    -- 下限变化量
    upper_change DOUBLE NOT NULL,    -- 上限变化量
    total_change DOUBLE NOT NULL,    -- 总变化量
    lower_change_ratio DOUBLE NOT NULL, -- 下限变化比例（百分比）
    upper_change_ratio DOUBLE NOT NULL, -- 上限变化比例（百分比）
    
    -- 指定时间索引
    TIME INDEX (`timestamp`),
    
    -- 设置标签（提高查询性能）
    PRIMARY KEY (task_id, factory_id, product_id, round)
);

-- 说明：
-- 1. timestamp作为时间索引，用于时序查询
-- 2. 标签字段选择了查询频率高的字段，提高过滤和分组性能
-- 3. 数值型字段作为普通字段，支持聚合分析
-- 4. 主键设计考虑了数据分布和查询模式
-- 5. GreptimeDB支持自动分区和压缩，适合存储大量时序数据

-- 示例插入语句
-- INSERT INTO factory_range_optimization_logs VALUES (
--     1630000000000, -- timestamp
--     100,           -- round
--     'task_123',    -- task_id
--     1,             -- factory_id
--     'factory_1',   -- factory_name
--     101,           -- product_id
--     'Success',     -- trade_result
--     100.0,         -- old_range_lower
--     200.0,         -- old_range_upper
--     101.0,         -- new_range_lower
--     201.0,         -- new_range_upper
--     1.0,           -- lower_change
--     1.0,           -- upper_change
--     2.0,           -- total_change
--     1.0,           -- lower_change_ratio
--     0.5            -- upper_change_ratio
-- );
