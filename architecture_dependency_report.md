# 奥地利市场模拟器 - 架构依赖分析报告

## 1. 包结构概览

| 包名 | 包含的核心类型 |
|------|---------------|
| `entity` | NormalDistribution |
| `logging` | LOGGER, Logger, MYSQL_POOL, AgentCashLog, AgentDemandRemovalLog ... (共 9 个) |
| `model` | Agent, IntervalRelation, TradeResult, Preference, Factory ... (共 11 个) |

## 2. 文件结构概览

| 文件路径 | 包含的核心类型 |
|---------|---------------|
| `src/entity/normal_distribute.rs` | NormalDistribution |
| `src/logging.rs` | LOGGER, Logger, MYSQL_POOL |
| `src/logging/agent_cash_log.rs` | AgentCashLog |
| `src/logging/agent_demand_removal_log.rs` | AgentDemandRemovalLog |
| `src/logging/agent_range_adjustment_log.rs` | AgentRangeAdjustmentLog |
| `src/logging/factory_end_of_round_log.rs` | FactoryEndOfRoundLog |
| `src/logging/factory_range_optimization_log.rs` | FactoryRangeOptimizationLog |
| `src/logging/trade_log.rs` | TradeLog |
| `src/model/agent.rs` | Agent, IntervalRelation, TradeResult |
| `src/model/agent/preference.rs` | Preference |
| `src/model/factory.rs` | Factory, FactoryStatus |
| `src/model/factory/accountant.rs` | Accountant |
| `src/model/factory/financial_bill.rs` | FinancialBill |
| `src/model/market.rs` | Market |
| `src/model/product.rs` | Product, ProductCategory |

## 3. 文件级别依赖关系

### entity.rs (src/entity.rs)

**包含 (owns):**
- `normal_distribute.rs` (src/entity/normal_distribute.rs)

### lib.rs (src/lib.rs)

**使用 (uses):**
- `normal_distribute.rs` (src/entity/normal_distribute.rs)
- `logging.rs` (src/logging.rs)
- `market.rs` (src/model/market.rs)
- `product.rs` (src/model/product.rs)

### logging.rs (src/logging.rs)

**包含 (owns):**
- `agent_cash_log.rs` (src/logging/agent_cash_log.rs)
- `agent_demand_removal_log.rs` (src/logging/agent_demand_removal_log.rs)
- `agent_range_adjustment_log.rs` (src/logging/agent_range_adjustment_log.rs)
- `factory_end_of_round_log.rs` (src/logging/factory_end_of_round_log.rs)
- `factory_range_optimization_log.rs` (src/logging/factory_range_optimization_log.rs)
- `trade_log.rs` (src/logging/trade_log.rs)

**使用 (uses):**
- `agent_cash_log.rs` (src/logging/agent_cash_log.rs)
- `agent_demand_removal_log.rs` (src/logging/agent_demand_removal_log.rs)
- `agent_range_adjustment_log.rs` (src/logging/agent_range_adjustment_log.rs)
- `factory_end_of_round_log.rs` (src/logging/factory_end_of_round_log.rs)
- `factory_range_optimization_log.rs` (src/logging/factory_range_optimization_log.rs)
- `trade_log.rs` (src/logging/trade_log.rs)
- `agent.rs` (src/model/agent.rs)
- `factory.rs` (src/model/factory.rs)
- `product.rs` (src/model/product.rs)

### agent_range_adjustment_log.rs (src/logging/agent_range_adjustment_log.rs)

**使用 (uses):**
- `agent.rs` (src/model/agent.rs)
- `product.rs` (src/model/product.rs)

### factory_range_optimization_log.rs (src/logging/factory_range_optimization_log.rs)

**使用 (uses):**
- `logging.rs` (src/logging.rs)

### trade_log.rs (src/logging/trade_log.rs)

**使用 (uses):**
- `agent.rs` (src/model/agent.rs)
- `factory.rs` (src/model/factory.rs)
- `product.rs` (src/model/product.rs)

### model.rs (src/model.rs)

**包含 (owns):**
- `agent.rs` (src/model/agent.rs)
- `factory.rs` (src/model/factory.rs)
- `market.rs` (src/model/market.rs)
- `math.rs` (src/model/math.rs)
- `product.rs` (src/model/product.rs)

### agent.rs (src/model/agent.rs)

**包含 (owns):**
- `preference.rs` (src/model/agent/preference.rs)

**使用 (uses):**
- `logging.rs` (src/logging.rs)
- `preference.rs` (src/model/agent/preference.rs)
- `factory.rs` (src/model/factory.rs)
- `math.rs` (src/model/math.rs)
- `product.rs` (src/model/product.rs)

### preference.rs (src/model/agent/preference.rs)

**使用 (uses):**
- `normal_distribute.rs` (src/entity/normal_distribute.rs)
- `agent.rs` (src/model/agent.rs)
- `product.rs` (src/model/product.rs)

### factory.rs (src/model/factory.rs)

**包含 (owns):**
- `accountant.rs` (src/model/factory/accountant.rs)
- `financial_bill.rs` (src/model/factory/financial_bill.rs)

**使用 (uses):**
- `normal_distribute.rs` (src/entity/normal_distribute.rs)
- `logging.rs` (src/logging.rs)
- `factory_range_optimization_log.rs` (src/logging/factory_range_optimization_log.rs)
- `agent.rs` (src/model/agent.rs)
- `accountant.rs` (src/model/factory/accountant.rs)
- `financial_bill.rs` (src/model/factory/financial_bill.rs)
- `math.rs` (src/model/math.rs)
- `product.rs` (src/model/product.rs)

### accountant.rs (src/model/factory/accountant.rs)

**使用 (uses):**
- `financial_bill.rs` (src/model/factory/financial_bill.rs)

### market.rs (src/model/market.rs)

**使用 (uses):**
- `logging.rs` (src/logging.rs)
- `agent.rs` (src/model/agent.rs)
- `preference.rs` (src/model/agent/preference.rs)
- `factory.rs` (src/model/factory.rs)
- `financial_bill.rs` (src/model/factory/financial_bill.rs)
- `math.rs` (src/model/math.rs)
- `product.rs` (src/model/product.rs)

### product.rs (src/model/product.rs)

**使用 (uses):**
- `normal_distribute.rs` (src/entity/normal_distribute.rs)

## 4. 文件依赖关系图 (Mermaid)

```mermaid
flowchart TD
    classDef file fill:#f9f,stroke:#333,stroke-width:2px;
    file_0_factory.rs["factory.rs"]
    file_1_preference.rs["preference.rs"]
    file_2_model.rs["model.rs"]
    file_3_trade_log.rs["trade_log.rs"]
    file_4_agent_demand_removal_log.rs["agent_demand_removal_log.rs"]
    file_5_lib.rs["lib.rs"]
    file_6_entity.rs["entity.rs"]
    file_7_product.rs["product.rs"]
    file_8_math.rs["math.rs"]
    file_9_factory_end_of_round_log.rs["factory_end_of_round_log.rs"]
    file_10_agent.rs["agent.rs"]
    file_11_market.rs["market.rs"]
    file_12_agent_cash_log.rs["agent_cash_log.rs"]
    file_13_logging.rs["logging.rs"]
    file_14_financial_bill.rs["financial_bill.rs"]
    file_15_normal_distribute.rs["normal_distribute.rs"]
    file_16_agent_range_adjustment_log.rs["agent_range_adjustment_log.rs"]
    file_17_accountant.rs["accountant.rs"]
    file_18_factory_range_optimization_log.rs["factory_range_optimization_log.rs"]
    file_6_entity.rs -->|owns| file_15_normal_distribute.rs
    file_5_lib.rs -.->|uses| file_13_logging.rs
    file_5_lib.rs -.->|uses| file_7_product.rs
    file_5_lib.rs -.->|uses| file_11_market.rs
    file_5_lib.rs -.->|uses| file_15_normal_distribute.rs
    file_13_logging.rs -->|owns| file_9_factory_end_of_round_log.rs
    file_13_logging.rs -->|owns| file_16_agent_range_adjustment_log.rs
    file_13_logging.rs -->|owns| file_3_trade_log.rs
    file_13_logging.rs -->|owns| file_12_agent_cash_log.rs
    file_13_logging.rs -->|owns| file_4_agent_demand_removal_log.rs
    file_13_logging.rs -->|owns| file_18_factory_range_optimization_log.rs
    file_13_logging.rs -.->|uses| file_7_product.rs
    file_13_logging.rs -.->|uses| file_9_factory_end_of_round_log.rs
    file_13_logging.rs -.->|uses| file_0_factory.rs
    file_13_logging.rs -.->|uses| file_10_agent.rs
    file_13_logging.rs -.->|uses| file_16_agent_range_adjustment_log.rs
    file_13_logging.rs -.->|uses| file_3_trade_log.rs
    file_13_logging.rs -.->|uses| file_12_agent_cash_log.rs
    file_13_logging.rs -.->|uses| file_4_agent_demand_removal_log.rs
    file_13_logging.rs -.->|uses| file_18_factory_range_optimization_log.rs
    file_16_agent_range_adjustment_log.rs -.->|uses| file_7_product.rs
    file_16_agent_range_adjustment_log.rs -.->|uses| file_10_agent.rs
    file_18_factory_range_optimization_log.rs -.->|uses| file_13_logging.rs
    file_3_trade_log.rs -.->|uses| file_7_product.rs
    file_3_trade_log.rs -.->|uses| file_0_factory.rs
    file_3_trade_log.rs -.->|uses| file_10_agent.rs
    file_2_model.rs -->|owns| file_7_product.rs
    file_2_model.rs -->|owns| file_8_math.rs
    file_2_model.rs -->|owns| file_0_factory.rs
    file_2_model.rs -->|owns| file_10_agent.rs
    file_2_model.rs -->|owns| file_11_market.rs
    file_10_agent.rs -->|owns| file_1_preference.rs
    file_10_agent.rs -.->|uses| file_7_product.rs
    file_10_agent.rs -.->|uses| file_8_math.rs
    file_10_agent.rs -.->|uses| file_0_factory.rs
    file_10_agent.rs -.->|uses| file_1_preference.rs
    file_10_agent.rs -.->|uses| file_13_logging.rs
    file_1_preference.rs -.->|uses| file_7_product.rs
    file_1_preference.rs -.->|uses| file_10_agent.rs
    file_1_preference.rs -.->|uses| file_15_normal_distribute.rs
    file_0_factory.rs -->|owns| file_14_financial_bill.rs
    file_0_factory.rs -->|owns| file_17_accountant.rs
    file_0_factory.rs -.->|uses| file_7_product.rs
    file_0_factory.rs -.->|uses| file_8_math.rs
    file_0_factory.rs -.->|uses| file_10_agent.rs
    file_0_factory.rs -.->|uses| file_17_accountant.rs
    file_0_factory.rs -.->|uses| file_18_factory_range_optimization_log.rs
    file_0_factory.rs -.->|uses| file_13_logging.rs
    file_0_factory.rs -.->|uses| file_14_financial_bill.rs
    file_0_factory.rs -.->|uses| file_15_normal_distribute.rs
    file_17_accountant.rs -.->|uses| file_14_financial_bill.rs
    file_11_market.rs -.->|uses| file_7_product.rs
    file_11_market.rs -.->|uses| file_8_math.rs
    file_11_market.rs -.->|uses| file_0_factory.rs
    file_11_market.rs -.->|uses| file_10_agent.rs
    file_11_market.rs -.->|uses| file_1_preference.rs
    file_11_market.rs -.->|uses| file_13_logging.rs
    file_11_market.rs -.->|uses| file_14_financial_bill.rs
    file_7_product.rs -.->|uses| file_15_normal_distribute.rs
```

## 5. 核心类依赖详情

### AgentRangeAdjustmentLog

**使用 (uses):**
- `Agent` (model)

### Agent

**使用 (uses):**
- `Preference` (model)
- `ProductCategory` (model)

### Preference

**使用 (uses):**
- `Agent` (model)

### Factory

**使用 (uses):**
- `FactoryStatus` (model)
- `Accountant` (model)
- `ProductCategory` (model)

### Accountant

**使用 (uses):**
- `FinancialBill` (model)

### Market

**使用 (uses):**
- `Agent` (model)
- `Factory` (model)
- `Product` (model)

### Product

**使用 (uses):**
- `NormalDistribution` (entity)
- `ProductCategory` (model)

## 6. 类依赖关系图 (Mermaid)

```mermaid
flowchart TD
    classDef struct fill:#f9f,stroke:#333,stroke-width:2px;
    classDef enum fill:#fcf,stroke:#333,stroke-width:2px;
    classDef trait fill:#cff,stroke:#333,stroke-width:2px;
    class_0_NormalDistribution["NormalDistribution"]
    class_1_LOGGER["LOGGER"]
    class_2_Logger["Logger"]
    class_3_MYSQL_POOL["MYSQL_POOL"]
    class_4_AgentCashLog["AgentCashLog"]
    class_5_AgentDemandRemovalLog["AgentDemandRemovalLog"]
    class_6_AgentRangeAdjustmentLog["AgentRangeAdjustmentLog"]
    class_7_FactoryEndOfRoundLog["FactoryEndOfRoundLog"]
    class_8_FactoryRangeOptimizationLog["FactoryRangeOptimizationLog"]
    class_9_TradeLog["TradeLog"]
    class_10_Agent["Agent"]
    class_11_IntervalRelation["IntervalRelation"]
    class_12_TradeResult["TradeResult"]
    class_13_Preference["Preference"]
    class_14_Factory["Factory"]
    class_15_FactoryStatus["FactoryStatus"]
    class_16_Accountant["Accountant"]
    class_17_FinancialBill["FinancialBill"]
    class_18_Market["Market"]
    class_19_Product["Product"]
    class_20_ProductCategory["ProductCategory"]
    class_6_AgentRangeAdjustmentLog -.->|use| class_10_Agent
    class_10_Agent -.->|use| class_13_Preference
    class_10_Agent -.->|use| class_20_ProductCategory
    class_13_Preference -.->|use| class_10_Agent
    class_14_Factory -.->|use| class_15_FactoryStatus
    class_14_Factory -.->|use| class_20_ProductCategory
    class_14_Factory -.->|use| class_16_Accountant
    class_16_Accountant -.->|use| class_17_FinancialBill
    class_18_Market -.->|use| class_10_Agent
    class_18_Market -.->|use| class_14_Factory
    class_18_Market -.->|use| class_19_Product
    class_19_Product -.->|use| class_20_ProductCategory
    class_19_Product -.->|use| class_0_NormalDistribution
```

## 7. 循环依赖分析

### 文件级循环依赖

**循环 1:**
`logging.rs` → `factory.rs` → `agent.rs` → `logging.rs`

**循环 2:**
`factory.rs` → `agent.rs` → `factory.rs`

**循环 3:**
`agent.rs` → `preference.rs` → `agent.rs`

**循环 4:**
`logging.rs` → `factory.rs` → `factory_range_optimization_log.rs` → `logging.rs`

**循环 5:**
`logging.rs` → `factory.rs` → `logging.rs`

### 类级循环依赖

**循环 1:**
`Agent` → `Preference` → `Agent`

## 8. 模块结构总结

```
austrian_market_sim
├── src
│   ├── entity
│   │   └── normal_distribute.rs
│   ├── logging
│   │   ├── agent_cash_log.rs
│   │   ├── agent_demand_removal_log.rs
│   │   ├── agent_range_adjustment_log.rs
│   │   ├── factory_end_of_round_log.rs
│   │   ├── factory_range_optimization_log.rs
│   │   ├── trade_log.rs
│   │   └── mod.rs
│   ├── model
│   │   ├── agent
│   │   │   ├── preference.rs
│   │   │   └── mod.rs
│   │   ├── factory
│   │   │   ├── accountant.rs
│   │   │   ├── financial_bill.rs
│   │   │   └── mod.rs
│   │   ├── market.rs
│   │   ├── math.rs
│   │   ├── product.rs
│   │   └── mod.rs
│   ├── util
│   │   └── mod.rs
│   ├── lib.rs
│   └── main.rs
```
