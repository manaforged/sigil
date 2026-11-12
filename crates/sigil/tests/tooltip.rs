use sigil::prelude::*;
use sigil::reactive::Owner;
use sigil_kittest::Harness;

#[test]
fn a_tooltip_over_the_next_button_does_not_take_its_click() {
    Owner::scope("tooltip-rail", || {
        let threads = signal(0_u32);
        let team = signal(0_u32);
        let mut ui = Harness::new(move || {
            column()
                .gap(Space::Xs)
                .child(text("Rail"))
                .child(text("Icons"))
                .child(
                    tooltip("A long name for threads").child(
                        button("T")
                            .ghost()
                            .square()
                            .activate(move || threads.update(|n| *n += 1))
                            .accessible("Threads"),
                    ),
                )
                .child(
                    tooltip("A long name for team").child(
                        button("M")
                            .ghost()
                            .square()
                            .activate(move || team.update(|n| *n += 1))
                            .accessible("Team"),
                    ),
                )
        })
        .pixels(240, 240);
        ui.click("Team");
        ui.click("Threads");
        assert_eq!((team.peek(), threads.peek()), (1, 1));
    });
}
