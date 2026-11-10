use super::*;

#[test]
fn for_each_renders_a_row_once_for_each_new_key() {
    Owner::scope("sigil_kernel_for_each_once", || {
        let items = signal(vec!["a", "b", "c"]);
        let renders = Rc::new(Cell::new(0));
        let counted = Rc::clone(&renders);
        let mut cx = Cx::new_root();
        let list = cx.add(for_each(
            items,
            |label: &&str| *label,
            move |label: &&str| {
                counted.set(counted.get() + 1);
                text(*label)
            },
        ));
        let steps: [(Vec<&str>, usize); 5] = [
            (vec!["a", "b", "c"], 3),
            (vec!["a", "b", "c", "d"], 4),
            (vec!["d", "c", "b", "a"], 4),
            (vec!["d", "c", "a"], 4),
            (vec!["d", "c", "a", "b"], 5),
        ];
        for (shown, rendered) in steps {
            items.set(shown.clone());
            cx.flush_reactive_children();
            let texts: Vec<String> = child_ids(&cx, list)
                .into_iter()
                .map(|id| text_value(&cx, id))
                .collect();
            assert_eq!(texts, shown);
            assert_eq!(renders.get(), rendered, "renders after {shown:?}");
        }
    });
}
