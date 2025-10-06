# TrueMan2 - High-Performance Cryptocurrency Exchange

A modern, scalable cryptocurrency exchange platform built with Rust and Next.js, featuring real-time trading, order matching, and WebSocket-based market data streaming.

## 🏗️ Architecture Overview

TrueMan2 follows a microservices architecture designed for high performance and scalability:
<img width="1288" height="616" alt="image" src="https://github.com/user-attachments/assets/477df56b-710c-4814-892d-0c98b135ce08" />

## 🚀 Core Services

trueman2/
  ├── api/              # Instead of /api
  ├── engine/           # Instead of /simulator  
  ├── ws/               # Instead of /engine
  ├── db-updater/       # Instead of /ws
  ├── simulator/        # Instead of /db-updater
  └── client/           # Instead of /db-updater


### 1. **API Service** (`/api`)
- **Framework**: Actix Web (Rust)
- **Purpose**: REST API endpoints for trading operations
- **Key Features**:
  - User authentication with JWT
  - Order management (create, cancel, query)
  - Balance operations (deposit, withdraw)
  - Admin panel for market/token management
  - Real-time order processing via Redis

### 2. **Trading Engine** (`/engine`)
- **Framework**: Tokio async runtime
- **Purpose**: Core order matching and trade execution
- **Key Features**:
  - In-memory order book management
  - Real-time order matching (FIFO algorithm)
  - Market and limit order support
  - Balance validation and locking
  - Trade execution and settlement

### 3. **WebSocket Service** (`/ws`)
- **Framework**: Axum (Rust)
- **Purpose**: Real-time market data streaming
- **Key Features**:
  - Live order book updates
  - Real-time trade notifications
  - Market ticker updates
  - Client subscription management

### 4. **Database Updater** (`/db-updater`)
- **Purpose**: Asynchronous database persistence
- **Key Features**:
  - Processes Redis stream events
  - Maintains data consistency
  - Handles order/trade/balance updates
  - Transaction-based operations

### 5. **Simulator** (`/simulator`)
- **Purpose**: Market simulation and stress testing
- **Key Features**:
  - Automated trading bots
  - Market data generation
  - Performance testing
  - Liquidity simulation

### 6. **Frontend** (`/client`)
- **Framework**: Next.js 15 with TypeScript
- **UI Library**: Radix UI + Tailwind CSS
- **State Management**: Zustand
- **Key Features**:
  - Modern trading interface
  - Real-time charts and order books
  - Responsive design
  - Dark/light theme support

## 🛠️ Technology Stack

### Backend (Rust)
- **Actix Web**: High-performance HTTP server
- **Tokio**: Async runtime for concurrency
- **Diesel**: Type-safe SQL ORM
- **Redis**: Message queuing and caching
- **JWT**: Stateless authentication
- **UUID**: Unique identifiers

### Frontend (TypeScript)
- **Next.js 15**: React framework with SSR
- **Tailwind CSS**: Utility-first styling
- **Radix UI**: Accessible component primitives
- **Zustand**: Lightweight state management
- **Axios**: HTTP client

### Infrastructure
- **PostgreSQL**: Primary database
- **Redis**: Message broker and cache
- **Docker**: Containerization
- **Docker Compose**: Multi-service orchestration