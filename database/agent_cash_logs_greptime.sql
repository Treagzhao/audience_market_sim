-- GreptimeDB建表语句 for agent_cash_logs
CREATE TABLE agent_cash_logs (
    -- 时间索引字段
    `timestamp` TIMESTAMP NOT NULL,
    
    -- 标签字段（用于分组、过滤的高频查询字段）
    round BIGINT,                    -- 模拟轮次
    task_id STRING,                  -- 任务ID
    agent_id BIGINT,                 -- 主体ID
    agent_name STRING,               -- 主体名称
    
    -- 字段（数值型数据，用于聚合分析）
    cash DOUBLE NOT NULL,            -- 主体现金
    total_trades BIGINT NOT NULL,     -- 累计交易数
    
    -- 指定时间索引
    TIME INDEX (`timestamp`),
    
    -- 设置标签（提高查询性能）
    PRIMARY KEY (task_id, agent_id, round)
);

-- 说明：
-- 1. timestamp作为时间索引，用于时序查询和分析
-- 2. 标签字段选择了高频查询的维度字段，优化过滤和分组性能
-- 3. 数值型字段支持聚合分析，如平均现金、总交易数等
-- 4. 主键设计考虑了数据分布和查询模式，确保唯一性
-- 5. GreptimeDB自动分区和压缩，适合存储大量时序数据

-- 示例插入语句
-- INSERT INTO agent_cash_logs VALUES (
--     1630000000000, -- timestamp
--     100,           -- round
--     'task_123',    -- task_id
--     1001,          -- agent_id
--     'Consumer_1001', -- agent_name
--     5000.0,        -- cash
--     15             -- total_trades
-- );

-- 示例查询
-- 查询特定任务的主体平均现金趋势
-- SELECT 
--     round,
--     AVG(cash) as avg_cash,
--     SUM(total_trades) as total_trades
-- FROM agent_cash_logs
-- WHERE task_id = 'task_123'
-- GROUP BY round
-- ORDER BY round;

-- 查询特定主体的现金变化
-- SELECT 
--     round,
--     cash,
--     total_trades
-- FROM agent_cash_logs
-- WHERE agent_id = 1001
-- ORDER BY round;
