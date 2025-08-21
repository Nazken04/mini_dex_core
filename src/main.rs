use axum::{
    debug_handler,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub mod matching_engine;
use matching_engine::{OrderBook, Trade};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Order {
    pub id: Uuid,
    pub order_type: OrderType,
    pub side: Side,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderPayload {
    pub order_type: OrderType,
    pub side: Side,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
}

async fn health_check() -> StatusCode {
    StatusCode::OK
}

struct AppStateInner {
    order_book: Mutex<OrderBook>,
    db_pool: PgPool,
}

type AppState = Arc<AppStateInner>;

#[debug_handler]
async fn create_order(
    State(state): State<AppState>,
    Json(payload): Json<CreateOrderPayload>,
) -> Json<Vec<Trade>> {
    let order = Order {
        id: Uuid::new_v4(),
        order_type: payload.order_type,
        side: payload.side,
        price: payload.price,
        quantity: payload.quantity,
        timestamp: Utc::now(),
    };

    println!("New order received: {:?}", order);

    let trades = {
        let mut order_book = state.order_book.lock().unwrap();

        if let Some(mev_message) = order_book.detect_arbitrage(&order) {
            println!("--- MEV DETECTED ---");
            println!("{}", mev_message);
            println!("--------------------");
        }

        order_book.match_order(order.clone())
    };

    if !trades.is_empty() {
        println!("Trades executed: {:?}", trades);
        for trade in &trades {
            let result = sqlx::query!(
                "INSERT INTO trades (id, maker_order_id, taker_order_id, price, quantity, timestamp) VALUES ($1, $2, $3, $4, $5, $6)",
                Uuid::new_v4(), 
                trade.maker_order_id,
                trade.taker_order_id,
                trade.price,
                trade.quantity,
                trade.timestamp
            )
            .execute(&state.db_pool)
            .await;

            if let Err(e) = result {
                eprintln!("Failed to save trade to DB: {}", e);
            } else {
                println!("Successfully saved trade to DB.");
            }
        }
    }

    Json(trades)
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await
        .expect("Failed to create DB pool.");

    println!("Database connection pool established.");

    let app_state = Arc::new(AppStateInner {
        order_book: Mutex::new(OrderBook::new()),
        db_pool,
    });

    let app = Router::new()
        .route("/", get(health_check))
        .route("/order", post(create_order))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
        
    println!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}