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

#[test]
fn test_seed_determinism() {
    let messages_default = Scenario::TickStreaming.generate_messages();
    let messages_seed42 = Scenario::TickStreaming.generate_messages_with_seed(42);

    assert_eq!(messages_default.len(), messages_seed42.len());

    for (m1, m2) in messages_default.iter().zip(messages_seed42.iter()) {
        match (m1, m2) {
            (Message::Tick(t1), Message::Tick(t2)) => {
                assert_eq!(t1.instrument_id, t2.instrument_id);
                assert_eq!(t1.price, t2.price);
                assert_eq!(t1.quantity, t2.quantity);
                assert_eq!(t1.side, t2.side);
                assert_eq!(t1.trade_id, t2.trade_id);
            }
            _ => panic!("Message type mismatch"),
        }
    }
}

#[test]
fn test_seed_variation() {
    let messages_42 = Scenario::TickStreaming.generate_messages_with_seed(42);
    let messages_43 = Scenario::TickStreaming.generate_messages_with_seed(43);

    assert_eq!(messages_42.len(), messages_43.len());

    // First messages should differ with different seeds
    match (&messages_42[0], &messages_43[0]) {
        (Message::Tick(t1), Message::Tick(t2)) => {
            // At least one field should differ
            let differs = t1.instrument_id != t2.instrument_id
                || t1.price != t2.price
                || t1.quantity != t2.quantity
                || t1.trade_id != t2.trade_id;
            assert!(differs, "Different seeds should produce different messages");
        }
        _ => panic!("Expected Tick messages"),
    }
}

#[test]
fn test_burst_traffic_structure() {
    let messages = Scenario::BurstTraffic.generate_messages();
    assert_eq!(messages.len(), 1_000_000);

    let normal_size = 900_000;

    // Normal phase (first 900K): should contain all 5 message types
    let mut has_tick = false;
    let mut has_order = false;
    let mut has_book_small = false;
    let mut has_book_medium = false;
    let mut has_book_large = false;

    for msg in &messages[..normal_size] {
        match msg {
            Message::Tick(_) => has_tick = true,
            Message::Order(_) => has_order = true,
            Message::OrderBook(book) => {
                let levels = book.bids.len();
                if levels <= 5 {
                    has_book_small = true;
                } else if levels <= 20 {
                    has_book_medium = true;
                } else {
                    has_book_large = true;
                }
            }
        }
    }

    assert!(has_tick, "Normal phase should contain ticks");
    assert!(has_order, "Normal phase should contain orders");
    assert!(has_book_small, "Normal phase should contain small books");
    assert!(has_book_medium, "Normal phase should contain medium books");
    assert!(has_book_large, "Normal phase should contain large books");

    // Burst phase (last 100K): all ticks
    for msg in &messages[normal_size..] {
        assert!(
            matches!(msg, Message::Tick(_)),
            "Burst phase should contain only ticks"
        );
    }
}

#[test]
fn test_short_name_roundtrip() {
    for scenario in all_scenarios() {
        let short = scenario.short_name();
        let recovered = Scenario::from_short_name(short)
            .unwrap_or_else(|| panic!("from_short_name failed for '{}'", short));
        assert_eq!(scenario, recovered);
    }
}

