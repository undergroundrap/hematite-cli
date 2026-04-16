#[test]
fn debug_routing_collision() {
    use hematite::agent::routing::{all_host_inspection_topics, preferred_host_inspection_topic};
    let input = "Analyze the AD user administrator. Show their SID and group memberships.";
    
    let all = all_host_inspection_topics(input);
    println!("ALL TOPICS: {:?}", all);
    
    let preferred = preferred_host_inspection_topic(input);
    println!("PREFERRED TOPIC: {:?}", preferred);
    
    assert_eq!(preferred, Some("ad_user"), "Should prefer ad_user, but preferred {:?}", preferred);
}
