//! Loopback response held open until the cancellation test releases it.
//! A slow CI scheduler cannot accidentally observe an already-completed repair.
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread::{self, JoinHandle},
    time::Duration,
};

pub(crate) struct HeldDownload {
    pub url: String,
    stop: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
}
impl HeldDownload {
    pub fn new(body: Vec<u8>) -> Self {
        assert!(body.len() > 1);
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}/manifest.json", listener.local_addr().unwrap());
        let (stop, stopped) = mpsc::channel();
        let thread = thread::spawn(move || {
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        if stopped.recv_timeout(Duration::from_millis(5)).is_ok() {
                            return;
                        }
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            };
            // An accepted socket inherits the listener's non-blocking mode on
            // macOS, and the request may not have arrived yet.
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            let mut request = [0; 4096];
            let count = stream.read(&mut request).unwrap();
            assert!(request[..count].starts_with(b"GET /seed.zip "));
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )
            .unwrap();
            // Cross the production progress throttle while never sending EOF or
            // the full body. Cancellation therefore always happens precommit.
            let mut sent = 0;
            while stopped.recv_timeout(Duration::from_millis(50)).is_err() {
                if sent < body.len() - 1 {
                    if stream.write_all(&body[sent..sent + 1]).is_err() {
                        return;
                    }
                    sent += 1;
                }
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }
}
impl Drop for HeldDownload {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take() {
            if !std::thread::panicking() {
                thread.join().unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream;

    #[test]
    fn a_request_that_arrives_after_the_connection_is_still_served() {
        let download = HeldDownload::new(vec![7; 8]);
        let address = download
            .url
            .strip_prefix("http://")
            .and_then(|rest| rest.strip_suffix("/manifest.json"))
            .unwrap();
        let mut client = TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        // Long enough for the origin to accept and start reading an empty socket.
        thread::sleep(Duration::from_millis(200));
        client
            .write_all(b"GET /seed.zip HTTP/1.1\r\nHost: fixture\r\n\r\n")
            .unwrap();
        let mut response = [0; 15];
        client.read_exact(&mut response).unwrap();
        assert_eq!(&response, b"HTTP/1.1 200 OK");
    }
}
