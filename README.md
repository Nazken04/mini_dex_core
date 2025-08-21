
```markdown
# High-Performance Order Matching Engine in Rust

![Rust](https://img.shields.io/badge/Rust-2024%20Edition-darkorange?style=for-the-badge&logo=rust)
![Axum](https://img.shields.io/badge/Axum-Async%20API-blue?style=for-the-badge)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-Persistence-blue?style=for-the-badge&logo=postgresql)
![Docker](https://img.shields.io/badge/Docker-Containerized-blue?style=for-the-badge&logo=docker)

## Overview

This project is a high-performance, concurrent order matching engine—the core backend service of a financial exchange. Developed entirely in Rust, it provides a robust, low-latency solution for processing limit orders, maintaining an in-memory order book, and persisting executed trades to a PostgreSQL database.

Crucially, this system goes beyond standard exchange logic by implementing a simulation of **Maximal Extractable Value (MEV)**, specifically detecting arbitrage opportunities before they are executed. This demonstrates a deep understanding of the complex and adversarial environments found in Web3 and DeFi protocols.

---

## Architectural Overview

The system follows a clean, logical flow designed for performance and scalability.

```mermaid
graph TD
    A[Client/Trader] -- HTTP POST Request --> B{Axum REST API};
    B -- Submits Order --> C{Order Matching Engine};
    C -- Acquires Lock --> D[In-Memory Order Book (BTreeMap)];
    C -- 1. MEV Detection --> E((Log Arbitrage Opportunity));
    C -- 2. Match Logic --> F[Generate Trades];
    F -- Persist Trades --> G[(PostgreSQL Database)];
    F -- Return Executed Trades --> B;
    B -- HTTP 200 OK Response --> A;
```

---

## 🚀 Live Showcase: Detecting an Arbitrage Opportunity

This tutorial demonstrates the MEV detection feature in action.

### Step 1: Run the Project

(Prerequisites: Rust, Docker, `sqlx-cli` installed)

1.  **Clone & Set Up:**
    ```bash
    git clone <your-repo-link>
    cd <your-repo-name>
    # Create the .env file for local use
    echo "DATABASE_URL=postgres://postgres:postgres@localhost:5432/mini_dex" > .env
    ```
2.  **Start Database & Run Migrations:**
    ```bash
    docker-compose up -d db
    sqlx migrate run
    ```
3.  **Prepare Offline Data for Docker Build:**
    ```bash
    cargo sqlx prepare
    ```
4.  **Build & Launch the Full Application:**
    ```bash
    docker-compose up --build
    ```
    Wait for the log message `listening on 0.0.0.0:3000`. The system is now live.

### Step 2: The Scenario

We will create a scenario where a bot could perform arbitrage. Keep the Docker logs visible (`Terminal 1`) and open a new terminal (`Terminal 2`) to send requests.

#### Action 1: A Seller Places an Order

A seller places a limit order to sell 5 units at **$45,000**. This becomes the best available price (the "best ask").

*   **Terminal 2 (Client):**
    ```powershell
    Invoke-RestMethod -Uri http://127.0.0.1:3000/order `
      -Method Post `
      -Headers @{ "Content-Type" = "application/json" } `
      -Body '{
        "order_type": "Limit", "side": "Sell", "price": 45000.0, "quantity": 5.0
      }'
    ```

#### Action 2: An Eager Buyer Creates the Opportunity

An eager buyer submits a buy order for 1 unit, but is willing to pay up to **$45,100**. This is a clear arbitrage opportunity: a bot could buy from the seller at $45,000 and instantly sell to this buyer for a $100 profit. Our system will detect this.

*   **Terminal 2 (Client):**
    ```powershell
    Invoke-RestMethod -Uri http://127.0.0.1:3000/order `
      -Method Post `
      -Headers @{ "Content-Type" = "application/json" } `
      -Body '{
        "order_type": "Limit", "side": "Buy", "price": 45100.0, "quantity": 1.0
      }'
    ```

### Step 3: Verification - MEV Detected!

Instantly, the logs in **Terminal 1** will show the detection alert *before* the trade is processed.

    **Log Text:**
    ```
    mini_dex_core_app  | New order received: Order { ..., side: Buy, price: Some(45100.0), ...}
    mini_dex_core_app  | --- MEV DETECTED ---
    mini_dex_core_app  | Arbitrage: Incoming BUY order at 45100.0 is higher than best ASK of 45000.0. Opportunity to buy at 45000.0 and sell at 45100.0.
    mini_dex_core_app  | --------------------
    mini_dex_core_app  | Trades executed: [Trade { ..., price: 45000.0, quantity: 1.0, ... }]
    mini_dex_core_app  | Successfully saved trade to DB.
    ```
This showcase proves the system is not only functional as an exchange but also demonstrates the deeper, domain-specific awareness required for building robust Web3 applications.
```

