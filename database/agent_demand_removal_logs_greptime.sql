-- Agent需求删除日志表
CREATE TABLE IF NOT EXISTS agent_demand_removal_logs (
   `timestamp` timestamp NOT NULL,
    round BIGINT NOT NULL INVERTED index,
    task_id STRING NOT NULL INVERTED index,
    agent_id BIGINT NOT NULL INVERTED index,
    agent_name STRING NOT NULL,
    product_id BIGINT NOT NULL INVERTED index,
    agent_cash DOUBLE NOT NULL,
    agent_pref_original_price DOUBLE NULL,
    agent_pref_original_elastic DOUBLE NULL,
    agent_pref_current_price DOUBLE NULL,
    agent_pref_current_range_lower DOUBLE NULL,
    agent_pref_current_range_upper DOUBLE NULL,
    removal_reason STRING NOT NULL,
   TIME INDEX (`timestamp`),
    -- 设置标签（提高查询性能）
    PRIMARY KEY (task_id, agent_id, round)
);
