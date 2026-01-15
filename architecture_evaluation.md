# 架构健康度评估报告

## 规则 1.1.1：源代码分配给领域模块的百分比

### 评估标准
- >90% 得10分
- ≤54% 得0分
- 中间值按比例计算

### 领域模块识别

#### 业务领域模块（业务能力模块）
业务领域模块是直接反映业务现实的高内聚单元，包括：

| 模块名称 | 文件路径 | 有效代码行数 |
|---------|---------|-------------|
| Agent（代理人） | ./src/model/agent.rs | 423 |
| Agent Preference（代理人偏好） | ./src/model/agent/preference.rs | 51 |
| Factory（工厂） | ./src/model/factory.rs | 389 |
| Factory Accountant（工厂会计） | ./src/model/factory/accountant.rs | 68 |
| Factory Financial Bill（工厂财务账单） | ./src/model/factory/financial_bill.rs | 77 |
| Market（市场） | ./src/model/market.rs | 453 |
| Product（产品） | ./src/model/product.rs | 124 |
| Model Util（业务工具函数） | ./src/model/util.rs | 171 |

**业务领域模块总行数：** 423 + 51 + 389 + 68 + 77 + 453 + 124 + 171 = 1756 行

#### 非业务领域模块
非业务领域模块是支持业务运行的基础设施代码，包括：

| 模块名称 | 文件路径 | 有效代码行数 |
|---------|---------|-------------|
| Logging（日志系统） | ./src/logging.rs | 358 |
| Agent Cash Log（代理人现金日志） | ./src/logging/agent_cash_log.rs | 63 |
| Agent Demand Removal Log（代理人需求移除日志） | ./src/logging/agent_demand_removal_log.rs | 141 |
| Agent Range Adjustment Log（代理人价格范围调整日志） | ./src/logging/agent_range_adjustment_log.rs | 143 |
| Factory End Of Round Log（工厂轮次结束日志） | ./src/logging/factory_end_of_round_log.rs | 211 |
| Factory Range Optimization Log（工厂价格范围优化日志） | ./src/logging/factory_range_optimization_log.rs | 135 |
| Trade Log（交易日志） | ./src/logging/trade_log.rs | 174 |
| Normal Distribution（正态分布工具） | ./src/entity/normal_distribute.rs | 75 |
| Entity Module（实体模块） | ./src/entity.rs | 2 |
| Main（程序入口） | ./src/main.rs | 173 |

**非业务领域模块总行数：** 358 + 63 + 141 + 143 + 211 + 135 + 174 + 75 + 2 + 173 = 1475 行

### 计算结果

**总有效代码行数：** 3239 行

**领域模块代码占比：** (1756 / 3239) × 100% ≈ 54.2%

### 评分

根据规则 1.1.1，领域模块代码占比约为 54.2%，略高于 54%，因此得分为：1分

### 评估结论

该项目的领域模块代码占比约为 54.2%，勉强超过最低及格线（54%），架构的业务聚焦度有待提高。大部分非领域代码集中在日志系统（约 1298 行），这部分代码与业务逻辑分离较好，但占比较高。

建议：
1. 考虑进一步优化日志系统的代码结构，减少冗余
2. 确保新功能开发优先向领域模块集中，提高业务代码的内聚性
3. 定期评估代码分配，确保业务领域模块保持较高占比