use super::{GameKind, Validation};
use crate::minigame::games::{
    alignment, converse_humanoid, create, crystal_game, goauld_crystals, validation,
};
use crate::minigame::session::MinigameSession;

#[test]
fn every_kind_round_trips_through_its_name() {
    for kind in GameKind::ALL {
        assert_eq!(
            GameKind::from_name(kind.name()),
            Some(kind),
            "{}",
            kind.name()
        );
    }
}

#[test]
fn from_name_is_exact_and_case_sensitive() {
    assert_eq!(GameKind::from_name("livewire"), None);
    assert_eq!(GameKind::from_name("NoSuchGame"), None);
}

#[test]
fn validation_table_follows_the_port_flags() {
    assert_eq!(GameKind::Livewire.validation(), Validation::Server);
    let ports = [
        (GameKind::Alignment, alignment::PORTED),
        (GameKind::CrystalGame, crystal_game::PORTED),
        (GameKind::GoauldCrystals, goauld_crystals::PORTED),
        (GameKind::ConverseBasicHumanoid, converse_humanoid::PORTED),
    ];
    for (kind, ported) in ports {
        let expected = if ported {
            Validation::Server
        } else {
            Validation::ClientDeclared
        };
        assert_eq!(kind.validation(), expected, "{}", kind.name());
    }
    for kind in [
        GameKind::Hack,
        GameKind::Activate,
        GameKind::Analyze,
        GameKind::Bypass,
        GameKind::Converse,
    ] {
        assert_eq!(
            kind.validation(),
            Validation::ClientDeclared,
            "{}",
            kind.name()
        );
    }
    assert_eq!(Validation::Server.as_str(), "server");
    assert_eq!(Validation::ClientDeclared.as_str(), "client_declared");
    assert_eq!(validation("NoSuchGame"), "client_declared");
}

#[test]
fn create_resolves_every_kind_and_unknown_names() {
    for kind in GameKind::ALL {
        let session = MinigameSession::offline(kind.name(), 1, 7);
        assert!(create(&session).is_some(), "{}", kind.name());
    }
    let unknown = MinigameSession::offline("NoSuchGame", 1, 7);
    assert!(create(&unknown).is_some());
}

/// Livewire is the only game the server validates; every placeholder
/// name, and any unknown name, is client-declared.
#[test]
fn validation_names_the_validator() {
    assert_eq!(validation("Livewire"), "server");
    for name in [
        "Hack",
        "Activate",
        "Analyze",
        "Bypass",
        "Converse",
        "ConverseBasicHumanoid",
    ] {
        assert_eq!(validation(name), "client_declared", "{name}");
    }
    assert_eq!(validation("NoSuchGame"), "client_declared");
}
