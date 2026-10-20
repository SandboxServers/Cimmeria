use super::*;

async fn register(registry: &SessionRegistry) -> String {
    registry
        .register(
            77,
            1,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
            1,
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn cancel_if_ticket_only_removes_the_matching_launch() {
    let registry = SessionRegistry::new();
    let old = register(&registry).await;
    let claimed = registry
        .authenticate_and_claim(77, &old, "Livewire")
        .await
        .unwrap();
    let mut canceled = claimed.cancel.subscribe();
    assert!(!registry.cancel_if_ticket(77, "forged").await);
    assert!(!*canceled.borrow());
    assert!(registry.authenticate(77, &old, "Livewire").await.is_some());

    assert!(registry.cancel_if_ticket(77, &old).await);
    canceled.changed().await.unwrap();
    assert!(*canceled.borrow());
    assert_eq!(registry.session_count().await, 0);
    assert!(registry.board_snapshot(77).await.is_none());

    let new = register(&registry).await;
    assert_ne!(old, new);
    assert!(!registry.cancel_if_ticket(77, &old).await);
    assert_eq!(registry.session_count().await, 1);
    assert!(registry.authenticate(77, &new, "Livewire").await.is_some());
}
