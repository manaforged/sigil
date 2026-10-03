use super::*;
use sigil::events::{EventRouter, LogicalKey};
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;

fn laid_out(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
    layout
        .compute(cx, root, Size::fixed(320.0, 240.0))
        .expect("layout computes");
    layout
}

#[test]
fn key_hook_on_a_widget_builder_runs_once_on_the_widget() {
    sigil::reactive::Runtime::scope("sigil-widget-builder-key-hook", || {
        let hits = Rc::new(Cell::new(0_u32));
        let mut cx = Cx::new_root();
        let root = cx.add(button("Go").key(LogicalKey::character("x"), {
            let hits = Rc::clone(&hits);
            move |_| hits.set(hits.get() + 1)
        }));
        let layout = laid_out(&mut cx, root);

        let mut router = EventRouter::new();
        router.set_focus(&mut cx, Some(root));
        router.route(&mut cx, &layout, text_down("x"));

        assert_eq!(hits.get(), 1);
    });
}

#[test]
fn map_node_reports_the_root_id_of_a_list_and_an_image() {
    sigil::reactive::Runtime::scope("sigil-widget-builder-map-node", || {
        let list_id = Rc::new(Cell::new(None));
        let image_id = Rc::new(Cell::new(None));
        let mut cx = Cx::new_root();
        let list = cx.add(
            virtual_list(
                vec![String::from("a"), String::from("b")],
                |_, row: &String| list_row(false, row.clone(), ""),
            )
            .map_node({
                let list_id = Rc::clone(&list_id);
                move |node| node.on_added(move |_, id| list_id.set(Some(id)))
            }),
        );
        let pixels = sigil::kernel::ImageSource::from_rgba8(1, 1, vec![0; 4]).expect("pixels");
        let image = cx.add(image(pixels).map_node({
            let image_id = Rc::clone(&image_id);
            move |node| node.on_added(move |_, id| image_id.set(Some(id)))
        }));

        assert_eq!(list_id.get(), Some(list));
        assert_eq!(image_id.get(), Some(image));
    });
}
