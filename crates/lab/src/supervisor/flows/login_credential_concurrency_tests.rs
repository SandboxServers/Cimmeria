use super::*;
use tokio::sync::Notify;

struct ConcurrentCredentialIo<'a> {
    run: FlowRun<'a>,
    lane: &'static str,
    events: Arc<std::sync::Mutex<Vec<String>>>,
    password_entered: Arc<Notify>,
    release_password: Arc<Notify>,
    block_password: bool,
    fail_password: bool,
    account_mismatches: usize,
    retry_started: Arc<Notify>,
}

impl LoginCredentialIo for ConcurrentCredentialIo<'_> {
    async fn prepare_field(
        &mut self,
        step: &'static str,
        _window: &'static str,
    ) -> Result<(), FlowError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("{}:{step}", self.lane));
        Ok(())
    }

    async fn type_field(
        &mut self,
        step: &'static str,
        _window: &'static str,
        _text: &str,
    ) -> Result<(), FlowError> {
        self.events
            .lock()
            .unwrap()
            .push(format!("{}:{step}", self.lane));
        if step == "account" && self.account_mismatches > 0 {
            self.account_mismatches -= 1;
            return Err(self
                .run
                .fail(step, "account holds 1 characters after typing"));
        }
        if step == "password" && self.block_password {
            self.password_entered.notify_one();
            self.release_password.notified().await;
            if self.fail_password {
                return Err(self.run.fail(step, "password input failed"));
            }
        }
        Ok(())
    }

    fn record_retry(&mut self, _attempt: usize, _error: &FlowError) {
        self.events
            .lock()
            .unwrap()
            .push(format!("{}:retry", self.lane));
        self.retry_started.notify_one();
    }
}

async fn credential_lanes_are_serialized(fail_first: bool) {
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let password_entered = Arc::new(Notify::new());
    let release_password = Arc::new(Notify::new());
    let first_sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
    let second_sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;

    let first_events = events.clone();
    let first_entered = password_entered.clone();
    let first_release = release_password.clone();
    let first = tokio::spawn(async move {
        let mut io = ConcurrentCredentialIo {
            run: FlowRun::new(&first_sup, "lab_login"),
            lane: "first",
            events: first_events,
            password_entered: first_entered,
            release_password: first_release,
            block_password: true,
            fail_password: fail_first,
            account_mismatches: 0,
            retry_started: Arc::new(Notify::new()),
        };
        type_login_credentials(&mut io, "account", "secret").await
    });
    password_entered.notified().await;

    let second_events = events.clone();
    let second_started = Arc::new(Notify::new());
    let started = second_started.clone();
    let mut second = tokio::spawn(async move {
        started.notify_one();
        let mut io = ConcurrentCredentialIo {
            run: FlowRun::new(&second_sup, "lab_login"),
            lane: "second",
            events: second_events,
            password_entered: Arc::new(Notify::new()),
            release_password: Arc::new(Notify::new()),
            block_password: false,
            fail_password: false,
            account_mismatches: 0,
            retry_started: Arc::new(Notify::new()),
        };
        type_login_credentials(&mut io, "account2", "secret2").await
    });
    second_started.notified().await;
    // Give the second task a chance to contend while the first is suspended
    // in password input. It cannot even prepare its account field yet.
    assert!(tokio::time::timeout(Duration::from_millis(30), &mut second)
        .await
        .is_err());
    assert_eq!(
        *events.lock().unwrap(),
        [
            "first:account_focus",
            "first:account",
            "first:password_focus",
            "first:password"
        ]
    );
    release_password.notify_one();
    assert_eq!(first.await.unwrap().is_err(), fail_first);
    second.await.unwrap().unwrap();
    assert_eq!(
        *events.lock().unwrap(),
        [
            "first:account_focus",
            "first:account",
            "first:password_focus",
            "first:password",
            "second:account_focus",
            "second:account",
            "second:password_focus",
            "second:password"
        ]
    );
}

#[tokio::test]
async fn concurrent_login_lanes_wait_through_password() {
    credential_lanes_are_serialized(false).await;
}

#[tokio::test]
async fn failed_login_lane_releases_credential_lock() {
    credential_lanes_are_serialized(true).await;
}

#[tokio::test]
async fn concurrent_lane_waits_through_readback_retry_and_password() {
    let events = Arc::new(std::sync::Mutex::new(Vec::new()));
    let retry_started = Arc::new(Notify::new());
    let password_entered = Arc::new(Notify::new());
    let release_password = Arc::new(Notify::new());
    let first_sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;
    let second_sup = fake_bridge::supervisor(Arc::new(|_, _| Ok(Value::Null))).await;

    let first_events = events.clone();
    let first_retry = retry_started.clone();
    let first_password = password_entered.clone();
    let first_release = release_password.clone();
    let first = tokio::spawn(async move {
        let mut io = ConcurrentCredentialIo {
            run: FlowRun::new(&first_sup, "lab_login"),
            lane: "first",
            events: first_events,
            password_entered: first_password,
            release_password: first_release,
            block_password: true,
            fail_password: false,
            account_mismatches: 1,
            retry_started: first_retry,
        };
        type_login_credentials(&mut io, "account", "secret").await
    });
    retry_started.notified().await;

    let second_events = events.clone();
    let second_started = Arc::new(Notify::new());
    let started = second_started.clone();
    let mut second = tokio::spawn(async move {
        started.notify_one();
        let mut io = ConcurrentCredentialIo {
            run: FlowRun::new(&second_sup, "lab_login"),
            lane: "second",
            events: second_events,
            password_entered: Arc::new(Notify::new()),
            release_password: Arc::new(Notify::new()),
            block_password: false,
            fail_password: false,
            account_mismatches: 0,
            retry_started: Arc::new(Notify::new()),
        };
        type_login_credentials(&mut io, "account2", "secret2").await
    });
    second_started.notified().await;
    // The first lane is now in the retry delay. A lock scoped only to one
    // typing attempt would let the second lane prepare account here.
    assert!(tokio::time::timeout(Duration::from_millis(30), &mut second)
        .await
        .is_err());
    assert_eq!(
        *events.lock().unwrap(),
        ["first:account_focus", "first:account", "first:retry"]
    );

    password_entered.notified().await;
    assert_eq!(
        *events.lock().unwrap(),
        [
            "first:account_focus",
            "first:account",
            "first:retry",
            "first:account_focus",
            "first:account",
            "first:password_focus",
            "first:password"
        ]
    );
    release_password.notify_one();
    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();
    assert_eq!(
        &events.lock().unwrap()[7..],
        [
            "second:account_focus",
            "second:account",
            "second:password_focus",
            "second:password"
        ]
    );
}
