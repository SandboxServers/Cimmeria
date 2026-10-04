//! A withdrawal of consent while a request is in flight. The server holds its
//! answer; the opt-out is issued once the request is with the server, and the
//! cycle must end before the held answer could have arrived. The same run
//! without the opt-out, which waits for the answer and delivers, is the control.
use super::*;

/// How long the server holds its answer. A cycle that ends sooner cannot have
/// had it.
const HELD: Duration = Duration::from_secs(3);

async fn an_opt_out_aborts_a_held(route: &str) {
    for withdrawn in [false, true] {
        let rig = Rig::opted_in().await;
        let (mint, ingest) = (minted(), results(&["accepted"]));
        // The requests the server has received once `route` is the one pending.
        let sent = if route == MINT {
            mount(&rig.server, MINT, mint.set_delay(HELD)).await;
            mount(&rig.server, INGEST, ingest).await;
            vec![MINT]
        } else {
            mount(&rig.server, MINT, mint).await;
            mount(&rig.server, INGEST, ingest.set_delay(HELD)).await;
            vec![MINT, INGEST]
        };
        rig.fail(1);
        let generation = stored(&rig.owner()).generation;

        let started = std::time::Instant::now();
        let (outcome, ()) = tokio::join!(rig.cycle(), async {
            if withdrawn {
                assert_eq!(rig.paths_after(sent.len()).await, sent, "{route}");
                set_consent(&mut rig.owner(), false);
            }
        });
        let elapsed = started.elapsed();
        if !withdrawn {
            assert_eq!(outcome, DELIVERED_ONE, "{route}");
            assert!(elapsed >= HELD, "{route}: the answer really was held");
            assert_eq!(rig.paths().await, [MINT, INGEST], "{route}");
            continue;
        }
        assert_eq!(outcome, CycleOutcome::ConsentWithdrawn, "{route}");
        assert!(
            elapsed < HELD,
            "{route}: took {elapsed:?}, so the request was not aborted"
        );
        // Nothing followed the aborted request, and nothing waited.
        assert_eq!(rig.paths().await, sent, "{route}");
        assert_eq!(rig.probe.sleeps(), [], "{route}");
        assert_eq!(stored(&rig.owner()), Queue::empty(generation + 1));
    }
}

#[tokio::test]
async fn an_opt_out_aborts_a_held_mint() {
    an_opt_out_aborts_a_held(MINT).await;
}

#[tokio::test]
async fn an_opt_out_aborts_a_held_post() {
    an_opt_out_aborts_a_held(INGEST).await;
}
