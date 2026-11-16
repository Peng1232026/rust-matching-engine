use matching_engine::engine::Engine;
use matching_engine::order::{Order, Side};

fn buy(id: u64, price: i64, qty: u64) -> Order {
    Order::limit(id, Side::Buy, price, qty, id)
}

fn sell(id: u64, price: i64, qty: u64) -> Order {
    Order::limit(id, Side::Sell, price, qty, id)
}

#[test]
fn limit_buy_matches_best_ask() {
    let mut e = Engine::new();
    e.submit(sell(1, 100, 10));
    e.submit(buy(2, 100, 10));
    assert_eq!(e.trades().len(), 1);
    assert_eq!(e.trades()[0].price, 100);
    assert_eq!(e.trades()[0].qty, 10);
    assert_eq!(e.book().len(), 0);
}

#[test]
fn limit_sell_matches_best_bid() {
    let mut e = Engine::new();
    e.submit(buy(1, 100, 10));
    e.submit(sell(2, 99, 10));
    assert_eq!(e.trades().len(), 1);
    assert_eq!(e.trades()[0].price, 100);
    assert_eq!(e.book().len(), 0);
}

#[test]
fn price_priority() {
    let mut e = Engine::new();
    e.submit(sell(1, 101, 5));
    e.submit(sell(2, 100, 5));
    e.submit(buy(3, 101, 5));
    assert_eq!(e.trades().len(), 1);
    assert_eq!(e.trades()[0].maker_id, 2);
}

#[test]
fn time_priority() {
    let mut e = Engine::new();
    e.submit(sell(1, 100, 5));
    e.submit(sell(2, 100, 5));
    e.submit(buy(3, 100, 5));
    assert_eq!(e.trades().len(), 1);
    assert_eq!(e.trades()[0].maker_id, 1);
}

#[test]
fn partial_fill_leaves_remainder() {
    let mut e = Engine::new();
    e.submit(sell(1, 100, 5));
    e.submit(buy(2, 100, 12));
    assert_eq!(e.trades().len(), 1);
    assert_eq!(e.trades()[0].qty, 5);
    assert_eq!(e.book().len(), 1);
    assert_eq!(e.book().best_bid(), Some(100));
}

#[test]
fn cancel_removes_order() {
    let mut e = Engine::new();
    e.submit(sell(1, 100, 5));
    assert!(e.cancel(1));
    assert_eq!(e.book().len(), 0);
    assert!(!e.cancel(1));
}

#[test]
fn market_buy_takes_liquidity() {
    let mut e = Engine::new();
    e.submit(sell(1, 100, 5));
    e.submit(sell(2, 101, 5));
    e.submit(Order::market(3, Side::Buy, 7, 0));
    assert_eq!(e.trades().len(), 2);
    assert_eq!(e.book().len(), 1);
    assert_eq!(e.book().best_ask(), Some(101));
}

#[test]
fn no_cross_no_trade() {
    let mut e = Engine::new();
    e.submit(sell(1, 101, 5));
    e.submit(buy(2, 100, 5));
    assert_eq!(e.trades().len(), 0);
    assert_eq!(e.book().len(), 2);
}

#[test]
fn crossing_sweeps_multiple_levels() {
    let mut e = Engine::new();
    e.submit(sell(1, 100, 3));
    e.submit(sell(2, 101, 3));
    e.submit(sell(3, 102, 3));
    e.submit(buy(4, 101, 10));
    assert_eq!(e.trades().len(), 2);
    assert_eq!(e.book().len(), 2);
    assert_eq!(e.book().best_bid(), Some(101));
    assert_eq!(e.book().best_ask(), Some(102));
}
