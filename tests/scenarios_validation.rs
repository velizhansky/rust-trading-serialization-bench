use rust_trading_serialization_bench::evaluation::scenarios::{
    Scenario, Message, all_scenarios,
    MIXED_TICK_RATIO, MIXED_ORDER_RATIO, MIXED_BOOK_SMALL_RATIO, MIXED_BOOK_MEDIUM_RATIO,
};

#[test]
fn test_deterministic_generation() {
    let messages1 = Scenario::TickStreaming.generate_messages();
    let messages2 = Scenario::TickStreaming.generate_messages();
    
    assert_eq!(messages1.len(), messages2.len());
    
    for (m1, m2) in messages1.iter().zip(messages2.iter()) {
        match (m1, m2) {
            (Message::Tick(t1), Message::Tick(t2)) => {
                assert_eq!(t1.instrument_id, t2.instrument_id);
                assert_eq!(t1.price, t2.price);
                assert_eq!(t1.quantity, t2.quantity);
                assert_eq!(t1.side, t2.side);
            },
            _ => panic!("Message type mismatch"),
        }
    }
}

#[test]
fn test_sample_counts() {
    let scenarios = all_scenarios();
    
    for scenario in scenarios {
        let expected_count = scenario.sample_count();
        let messages = scenario.generate_messages();
        
        assert_eq!(
            messages.len(),
            expected_count,
            "Scenario {} should generate exactly {} messages",
            scenario.name(),
            expected_count
        );
    }
}

#[test]
fn test_mixed_workload_distribution() {
    let messages = Scenario::MixedWorkload.generate_messages();
    let total = messages.len() as f64;
    
    let mut tick_count = 0;
    let mut order_count = 0;
    let mut book_small_count = 0;
    let mut book_medium_count = 0;
    let mut book_large_count = 0;
    
    for msg in messages {
        match msg {
            Message::Tick(_) => tick_count += 1,
            Message::Order(_) => order_count += 1,
            Message::OrderBook(book) => {
                let levels = book.bids.len();
                if levels <= 5 {
                    book_small_count += 1;
                } else if levels <= 20 {
                    book_medium_count += 1;
                } else {
                    book_large_count += 1;
                }
            },
        }
    }
    
    let tick_ratio = tick_count as f64 / total;
    let order_ratio = order_count as f64 / total;
    let book_small_ratio = book_small_count as f64 / total;
    let book_medium_ratio = book_medium_count as f64 / total;
    let book_large_ratio = book_large_count as f64 / total;
    
    assert!(
        (tick_ratio - MIXED_TICK_RATIO).abs() < 0.05,
        "Tick ratio {:.2} should be close to {:.2}",
        tick_ratio,
        MIXED_TICK_RATIO
    );
    
    assert!(
        (order_ratio - MIXED_ORDER_RATIO).abs() < 0.05,
        "Order ratio {:.2} should be close to {:.2}",
        order_ratio,
        MIXED_ORDER_RATIO
    );
    
    assert!(
        (book_small_ratio - MIXED_BOOK_SMALL_RATIO).abs() < 0.02,
        "Small book ratio {:.2} should be close to {:.2}",
        book_small_ratio,
        MIXED_BOOK_SMALL_RATIO
    );
    
    assert!(
        (book_medium_ratio - MIXED_BOOK_MEDIUM_RATIO).abs() < 0.01,
        "Medium book ratio {:.2} should be close to {:.2}",
        book_medium_ratio,
        MIXED_BOOK_MEDIUM_RATIO
    );
    
    assert!(
        book_large_ratio < 0.02,
        "Large book ratio {:.2} should be less than 0.02",
        book_large_ratio
    );
}

