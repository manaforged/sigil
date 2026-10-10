#[test]
fn a_closed_peer_reads_as_a_hang_up() -> std::io::Result<()> {
    let (ours, theirs) = std::os::unix::net::UnixStream::pair()?;
    assert!(!super::hung_up(&ours));
    drop(theirs);
    assert!(super::hung_up(&ours));
    Ok(())
}

#[test]
fn a_hang_up_during_the_wait_ends_it() -> std::io::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;
    let hup = std::sync::Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGHUP, std::sync::Arc::clone(&hup))?;
    let (rx, _tx) = std::os::unix::net::UnixStream::pair()?;
    rx.set_nonblocking(true)?;
    let wake = super::Wake { rx: Some(rx) };
    let (ours, theirs) = std::os::unix::net::UnixStream::pair()?;
    let closer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        drop(theirs);
    });
    let wait = Duration::from_secs(5);
    let ready = wake.wait(&ours, (Some(wait), wait), true, |_| Ok(false))?;
    drop(closer.join());
    assert!(!ready);
    assert!(hup.load(Ordering::SeqCst), "no SIGHUP raised");
    Ok(())
}
