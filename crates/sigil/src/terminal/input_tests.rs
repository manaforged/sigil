#[test]
fn a_closed_peer_reads_as_a_hang_up() -> std::io::Result<()> {
    let (ours, theirs) = std::os::unix::net::UnixStream::pair()?;
    assert!(!super::hung_up(&ours));
    drop(theirs);
    assert!(super::hung_up(&ours));
    Ok(())
}
