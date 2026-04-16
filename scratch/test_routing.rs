fn main() {
    let input = "Is my GPU currently throttled and why?";
    let lower = input.to_lowercase();
    
    // Logic from routing.rs
    let asks_virtualization = lower.contains("gpu") || lower.contains("hardware");
    let asks_overclocker = lower.contains("overclocker")
        || lower.contains("gpu clock")
        || lower.contains("gpu throttle")
        || lower.contains("throttle reason")
        || lower.contains("root cause")
        || lower.contains("nvidia stats")
        || (lower.contains("gpu") && (lower.contains("throttle") || lower.contains("bottleneck") || lower.contains("performance")));
    
    println!("Query: {}", input);
    println!("asks_virtualization: {}", asks_virtualization);
    println!("asks_overclocker: {}", asks_overclocker);
    
    if asks_overclocker {
        println!("RESULT: overclocker");
    } else if asks_virtualization {
        println!("RESULT: hardware");
    } else {
        println!("RESULT: summary");
    }
}
