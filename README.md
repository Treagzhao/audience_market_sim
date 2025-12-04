# 🕯️ Praxeology Engine (Rust)

> *"It is not the task of economics to announce what men ought to do, but what they actually do."*  
> — **Ludwig von Mises**

## 📖 Overview

**Praxeology Engine** is an **Agent-Based Modeling (ABM)** simulation framework written in **Rust**, designed to explore the emergent properties of market processes from a micro-foundation of subjective value theory.

Unlike traditional macroeconomic models that rely on aggregate variables and equilibrium assumptions, this project simulates the economy from the bottom up. It instantiates individual **Agents** with subjective preferences, limited knowledge, and dynamic learning capabilities, allowing them to interact in a decentralized marketplace.

The goal is to computationally visualize key Austrian School concepts:
- **The Subjective Theory of Value**: Prices are not intrinsic but emerge from individual valuations.
- **Price Discovery Process**: How scattered knowledge coordinates through the price mechanism.
- **Cantillon Effects**: Simulating how monetary injection points distort relative price structures (Planned Phase).

## 🏗️ Architecture

The project leverages Rust's concurrency and safety features to simulate thousands of independent economic actors.

### Core Components

- **`Product`**: Represents a good in the market. It does not have a "true price" but rather a probability distribution of how it is perceived by the population (Price & Elasticity distributions).
- **`Agent`**: An autonomous economic actor. Each agent:
    - Holds a unique `Preference` for each product (collapsed from the product's distribution).
    - Manages a `Budget` and makes decisions based on marginal utility (implied).
    - **Learns** from market history to adjust price expectations dynamically.
- **`Market`**: The arena where Agents meet. (Currently implementing decentralized matching logic).

### Key Algorithms

#### 1. Subjective Valuation Instantiation
Agents do not perceive the "average" price. Each Agent generates their own `original_price` and `elasticity` based on the Product's distribution, creating a heterogeneous landscape of demand.

#### 2. Dynamic Expectation Adjustment (In Progress)
Agents adjust their bid/ask prices based on:
- **Success Rate**: The ratio of successful transactions in the last $N$ attempts.
- **Elasticity**: High elasticity agents are more sensitive to price changes and less likely to raise bids.

## 🚀 Getting Started

### Prerequisites
- Rust (stable channel)