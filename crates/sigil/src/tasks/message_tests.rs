use super::sealed::Sealed;

#[test]
fn std_receiver_delivers_in_order_then_reports_close() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("build tokio runtime");
    let (tx, mut rx) = std::sync::mpsc::channel();
    let sender = std::thread::spawn(move || {
        for n in 1..=3_u32 {
            tx.send(n).expect("receiver alive");
        }
    });
    let received = runtime.block_on(async {
        let mut received = Vec::new();
        while let Some(n) = Sealed::recv(&mut rx).await {
            received.push(n);
        }
        received
    });
    sender.join().expect("sender thread");
    assert_eq!(received, vec![1, 2, 3]);
    assert_eq!(Sealed::try_recv(&mut rx), None);
}
