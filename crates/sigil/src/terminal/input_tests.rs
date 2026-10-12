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

#[test]
fn a_hang_up_while_not_listening_ends_the_wait_without_the_event_reader() -> std::io::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};
    let hup = std::sync::Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGHUP, std::sync::Arc::clone(&hup))?;
    let wake = super::Wake { rx: None };
    let (ours, theirs) = std::os::unix::net::UnixStream::pair()?;
    let closer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        drop(theirs);
    });
    let wait = Duration::from_secs(5);
    let started = Instant::now();
    let blind_reader = |limit: Duration| {
        std::thread::sleep(limit);
        Ok(false)
    };
    let ready = wake.wait(&ours, (Some(wait), wait), false, blind_reader)?;
    drop(closer.join());
    assert!(!ready);
    assert!(hup.load(Ordering::SeqCst), "no SIGHUP raised");
    assert!(
        started.elapsed() < Duration::from_secs(2),
        "the wait ran to its limit"
    );
    Ok(())
}

#[test]
fn an_event_reader_at_end_of_file_reads_as_a_hang_up() -> std::io::Result<()> {
    use std::sync::atomic::{AtomicBool, Ordering};
    let hup = std::sync::Arc::new(AtomicBool::new(false));
    signal_hook::flag::register(signal_hook::consts::SIGHUP, std::sync::Arc::clone(&hup))?;
    let eof = || Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
    assert!(!super::ended(eof())?);
    assert!(hup.load(Ordering::SeqCst), "no SIGHUP raised");
    Ok(())
}
