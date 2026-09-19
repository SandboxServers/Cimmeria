//! What an anonymous peer can make the server log. WARN events are
//! forwarded to Discord, so nothing a scanner can trigger may log at WARN.

use super::*;

/// Drive one pre-login frame through [`handle_connection`] and return every
/// WARN-or-worse event it produced. WARN events are forwarded to Discord,
/// so anything an anonymous probe can trigger must stay below WARN.
async fn warnings_from_probe(frame: &str) -> Vec<crate::test_support::Captured> {
    let capture = crate::test_support::LogCapture::install();
    let (mut client, server, peer) = loopback_pair().await;
    let (tx, _rx) = mpsc::channel(16);
    let task = spawn_connection(server, peer, SessionRegistry::new(), tx, OBSERVE_WITHIN);
    send_null_terminated(&mut client, frame)
        .await
        .expect("probe send must succeed");
    assert!(
        server_hangs_up(&mut client, OBSERVE_WITHIN).await,
        "a frame that is not a valid handshake step must close the connection",
    );
    join_within(task, "probe").await;
    capture
        .all()
        .into_iter()
        .filter(|e| e.level <= tracing::Level::WARN)
        .collect()
}

/// An internet scanner speaking HTTP (or any non-SFS bytes) at the port is
/// expected noise. It must be dropped without a WARN, or every probe posts
/// to the Discord errors channel.
#[tokio::test]
async fn a_non_sfs_probe_is_dropped_without_a_warning() {
    let warnings = warnings_from_probe("GET / HTTP/1.1\r\nHost: example\r\n\r\n").await;
    assert!(
        warnings.is_empty(),
        "a non-SFS probe must not log at WARN or above; got {warnings:#?}",
    );
}

/// A well-formed SFS frame out of order — login before verChk — is just as
/// anonymous as garbage and gets the same quiet treatment.
#[tokio::test]
async fn an_out_of_order_handshake_frame_is_dropped_without_a_warning() {
    let warnings = warnings_from_probe(
        "<msg t='sys'><body action='login' r='0'><login z='Hack'>\
         <nick><![CDATA[1]]></nick><pword><![CDATA[x]]></pword></login></body></msg>",
    )
    .await;
    assert!(
        warnings.is_empty(),
        "an out-of-order handshake frame must not log at WARN or above; got {warnings:#?}",
    );
}
