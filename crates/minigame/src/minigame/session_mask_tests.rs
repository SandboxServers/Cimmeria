use super::*;

/// A game with no ported cheat bits stores no ability mask, whatever the
/// base sent. Revert the `&` in `register` and this stores 0x7FFF.
#[tokio::test]
async fn register_filters_the_mask_to_the_games_ability_bits() {
    let reg = SessionRegistry::new();
    let ticket = reg
        .register(
            42,
            1,
            "Livewire".into(),
            1,
            1,
            0,
            0x7FFF,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        )
        .await
        .unwrap();

    let session = reg.authenticate(42, &ticket, "Livewire").await.unwrap();
    assert_eq!(session.abilities_mask, 0);
}
