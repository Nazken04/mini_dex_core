
use crate::{Order, Side};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct Trade {
    pub maker_order_id: Uuid,
    pub taker_order_id: Uuid,
    pub price: Decimal,
    pub quantity: Decimal,
    pub timestamp: DateTime<Utc>,
}

pub struct OrderBook {
    pub bids: BTreeMap<Decimal, Vec<Order>>,
    pub asks: BTreeMap<Decimal, Vec<Order>>,
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub fn add_order(&mut self, order: Order) {
        if let Some(price) = order.price {
            match order.side {
                Side::Buy => {
                    self.bids.entry(price).or_default().push(order);
                }
                Side::Sell => {
                    self.asks.entry(price).or_default().push(order);
                }
            }
        }
    }

    pub fn detect_arbitrage(&self, new_order: &Order) -> Option<String> {
        let new_price = if let Some(p) = new_order.price { p } else { return None; };

        match new_order.side {
            Side::Buy => {
                if let Some((best_ask_price, _)) = self.asks.iter().next() {
                    if new_price > *best_ask_price {
                        return Some(format!(
                            "Arbitrage: Incoming BUY order at {} is higher than best ASK of {}. Opportunity to buy at {} and sell at {}.",
                            new_price, best_ask_price, best_ask_price, new_price
                        ));
                    }
                }
            }
            Side::Sell => {
                if let Some((best_bid_price, _)) = self.bids.iter().rev().next() {
                    if new_price < *best_bid_price {
                        return Some(format!(
                            "Arbitrage: Incoming SELL order at {} is lower than best BID of {}. Opportunity to buy at {} and sell at {}.",
                            new_price, best_bid_price, new_price, best_bid_price
                        ));
                    }
                }
            }
        }
        None
    }
    pub fn match_order(&mut self, mut taker_order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();

        let taker_price = match taker_order.price {
            Some(price) => price,
            None => {
                println!("Market orders not yet implemented.");
                return trades;
            }
        };

        match taker_order.side {
            Side::Buy => {
                let mut filled_ask_levels = Vec::new();

                for (&ask_price, orders_at_level) in self.asks.iter_mut() {
                    if taker_order.quantity == Decimal::ZERO {
                        break;
                    }
                    if ask_price > taker_price {
                        break;
                    }

                    let mut filled_maker_indices = Vec::new();
                    for (i, maker_order) in orders_at_level.iter_mut().enumerate() {
                        if taker_order.quantity == Decimal::ZERO {
                            break;
                        }

                        let trade_quantity = taker_order.quantity.min(maker_order.quantity);

                        trades.push(Trade {
                            maker_order_id: maker_order.id,
                            taker_order_id: taker_order.id,
                            price: maker_order.price.unwrap(),
                            quantity: trade_quantity,
                            timestamp: Utc::now(),
                        });

                        maker_order.quantity -= trade_quantity;
                        taker_order.quantity -= trade_quantity;

                        if maker_order.quantity == Decimal::ZERO {
                            filled_maker_indices.push(i);
                        }
                    }

                    for i in filled_maker_indices.into_iter().rev() {
                        orders_at_level.remove(i);
                    }

                    if orders_at_level.is_empty() {
                        filled_ask_levels.push(ask_price);
                    }
                }

                for price in filled_ask_levels {
                    self.asks.remove(&price);
                }
            }
            Side::Sell => {
                let mut filled_bid_levels = Vec::new();

                for (&bid_price, orders_at_level) in self.bids.iter_mut().rev() {
                    if taker_order.quantity == Decimal::ZERO {
                        break;
                    }
                    if bid_price < taker_price {
                        break;
                    }

                    let mut filled_maker_indices = Vec::new();
                    for (i, maker_order) in orders_at_level.iter_mut().enumerate() {
                        if taker_order.quantity == Decimal::ZERO {
                            break;
                        }

                        let trade_quantity = taker_order.quantity.min(maker_order.quantity);

                        trades.push(Trade {
                            maker_order_id: maker_order.id,
                            taker_order_id: taker_order.id,
                            price: maker_order.price.unwrap(),
                            quantity: trade_quantity,
                            timestamp: Utc::now(),
                        });

                        maker_order.quantity -= trade_quantity;
                        taker_order.quantity -= trade_quantity;

                        if maker_order.quantity == Decimal::ZERO {
                            filled_maker_indices.push(i);
                        }
                    }

                    for i in filled_maker_indices.into_iter().rev() {
                        orders_at_level.remove(i);
                    }

                    if orders_at_level.is_empty() {
                        filled_bid_levels.push(bid_price);
                    }
                }

                for price in filled_bid_levels {
                    self.bids.remove(&price);
                }
            }
        }

        if taker_order.quantity > Decimal::ZERO {
            self.add_order(taker_order);
        }

        trades
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Order, OrderType};
    use rust_decimal_macros::dec;

    fn create_test_order(side: Side, price: Decimal, quantity: Decimal) -> Order {
        Order {
            id: Uuid::new_v4(),
            order_type: OrderType::Limit,
            side,
            price: Some(price),
            quantity,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_arbitrage_detection_sell_side() {
        let mut order_book = OrderBook::new();
        order_book.add_order(create_test_order(Side::Buy, dec!(101.0), dec!(10.0)));

        let new_sell_order = create_test_order(Side::Sell, dec!(100.0), dec!(5.0));

        let mev = order_book.detect_arbitrage(&new_sell_order);
        assert!(mev.is_some());
        println!("Detected MEV: {}", mev.unwrap());
    }

    #[test]
    fn test_arbitrage_detection_buy_side() {
        let mut order_book = OrderBook::new();
        order_book.add_order(create_test_order(Side::Sell, dec!(100.0), dec!(10.0)));
        
        let new_buy_order = create_test_order(Side::Buy, dec!(101.0), dec!(5.0));

        let mev = order_book.detect_arbitrage(&new_buy_order);
        assert!(mev.is_some());
        println!("Detected MEV: {}", mev.unwrap());
    }

    #[test]
    fn test_no_arbitrage() {
        let mut order_book = OrderBook::new();
        order_book.add_order(create_test_order(Side::Buy, dec!(100.0), dec!(10.0)));
        let new_sell_order = create_test_order(Side::Sell, dec!(101.0), dec!(5.0));
        assert!(order_book.detect_arbitrage(&new_sell_order).is_none());
    }

    #[test]
    fn test_add_order() {
        let mut order_book = OrderBook::new();
        let buy_order = create_test_order(Side::Buy, dec!(100.0), dec!(10.0));
        let sell_order = create_test_order(Side::Sell, dec!(101.0), dec!(5.0));

        order_book.add_order(buy_order);
        order_book.add_order(sell_order);

        assert_eq!(order_book.bids.len(), 1);
        assert_eq!(order_book.asks.len(), 1);
        assert_eq!(
            order_book.bids.get(&dec!(100.0)).unwrap()[0].quantity,
            dec!(10.0)
        );
        assert_eq!(
            order_book.asks.get(&dec!(101.0)).unwrap()[0].quantity,
            dec!(5.0)
        );
    }

    #[test]
    fn test_simple_match_full_fill() {
        let mut order_book = OrderBook::new();
        let sell_maker = create_test_order(Side::Sell, dec!(100.0), dec!(10.0));
        order_book.add_order(sell_maker);

        let buy_taker = create_test_order(Side::Buy, dec!(100.0), dec!(10.0));
        let trades = order_book.match_order(buy_taker);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, dec!(10.0));
        assert_eq!(trades[0].price, dec!(100.0));
        assert!(order_book.asks.is_empty());
        assert!(order_book.bids.is_empty());
    }

    #[test]
    fn test_simple_match_partial_fill_of_maker() {
        let mut order_book = OrderBook::new();
        let sell_maker = create_test_order(Side::Sell, dec!(100.0), dec!(10.0));
        order_book.add_order(sell_maker);

        let buy_taker = create_test_order(Side::Buy, dec!(100.0), dec!(5.0));
        let trades = order_book.match_order(buy_taker);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, dec!(5.0));
        assert_eq!(
            order_book.asks.get(&dec!(100.0)).unwrap()[0].quantity,
            dec!(5.0)
        );
        assert!(order_book.bids.is_empty());
    }

    #[test]
    fn test_partial_fill_of_taker() {
        let mut order_book = OrderBook::new();
        let sell_maker = create_test_order(Side::Sell, dec!(100.0), dec!(10.0));
        order_book.add_order(sell_maker);

        let buy_taker = create_test_order(Side::Buy, dec!(100.0), dec!(15.0));
        let trades = order_book.match_order(buy_taker);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].quantity, dec!(10.0));
        assert!(order_book.asks.is_empty()); 
        assert_eq!(
            order_book.bids.get(&dec!(100.0)).unwrap()[0].quantity,
            dec!(5.0)
        ); 
    }

    #[test]
    fn test_multi_level_match() {
        let mut order_book = OrderBook::new();
        order_book.add_order(create_test_order(Side::Sell, dec!(100.0), dec!(5.0)));
        order_book.add_order(create_test_order(Side::Sell, dec!(101.0), dec!(5.0)));

        let buy_taker = create_test_order(Side::Buy, dec!(101.0), dec!(8.0));
        let trades = order_book.match_order(buy_taker);

        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].price, dec!(100.0));
        assert_eq!(trades[0].quantity, dec!(5.0));
        assert_eq!(trades[1].price, dec!(101.0));
        assert_eq!(trades[1].quantity, dec!(3.0));

        assert!(order_book.bids.is_empty());
        assert_eq!(order_book.asks.len(), 1);
        assert_eq!(
            order_book.asks.get(&dec!(101.0)).unwrap()[0].quantity,
            dec!(2.0)
        );
    }

    #[test]
    fn test_no_match() {
        let mut order_book = OrderBook::new();
        order_book.add_order(create_test_order(Side::Sell, dec!(101.0), dec!(10.0)));

        let buy_taker = create_test_order(Side::Buy, dec!(100.0), dec!(10.0));
        let trades = order_book.match_order(buy_taker);

        assert!(trades.is_empty());
        assert_eq!(order_book.bids.len(), 1); 
        assert_eq!(order_book.asks.len(), 1);
    }
}