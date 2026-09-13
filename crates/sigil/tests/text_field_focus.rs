#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::panic)]

use sigil::hal::KeyEventInit;
use sigil::prelude::*;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
    }
}

fn compute(cx: &mut Cx, root: NodeId) -> LayoutEngine {
    let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    layout
        .compute(cx, root, Size::fixed(320.0, 120.0))
        .expect("layout computes");
    layout
}

fn key(ch: &str, physical: PhysicalKey, phase: KeyPhase) -> ShellEvent {
    ShellEvent::Key(KeyEvent::new(KeyEventInit {
        key: LogicalKey::character(ch),
        physical,
        modifiers: ModifiersState::empty(),
        repeat: false,
        phase,
        text: (phase == KeyPhase::Down).then(|| ch.into()),
    }))
}

fn field_node(cx: &Cx, root: NodeId) -> NodeId {
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        let node = cx.get(id).expect("node alive");
        if node.a11y.role == Role::TextInput {
            return id;
        }
        stack.extend(node.children().iter().copied());
    }
    panic!("no text field in the tree");
}

#[test]
fn focus_when_bump_focuses_the_field() {
    sigil::reactive::Runtime::scope("text-field-focus-when", || {
        let value = signal(String::new());
        let trigger = signal(0_u64);
        let mut cx = Cx::new_root();
        let root = cx.add(text_field(value).focus_when(trigger));
        let layout = compute(&mut cx, root);
        let mut router = EventRouter::new();

        router.flush_focus_requests(&mut cx, &layout);
        assert_eq!(router.focus(), None);

        trigger.set(1);
        router.flush_focus_requests(&mut cx, &layout);
        assert_eq!(router.focus(), Some(root));
    });
}

#[test]
fn slash_shortcut_focuses_field_then_typing_reaches_it() {
    sigil::reactive::Runtime::scope("text-field-slash-e2e", || {
        let value = signal(String::new());
        let trigger = signal(0_u64);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(320.0, 120.0))
                .keyboard_shortcut(Shortcut::plain(LogicalKey::character("/")))
                .key(Shortcut::plain(LogicalKey::character("/")), move |_| {
                    trigger.set(trigger.peek() + 1);
                })
                .child(text_field(value).focus_when(trigger)),
        );
        let layout = compute(&mut cx, root);
        let field = field_node(&cx, root);
        let mut router = EventRouter::new();

        router.route(
            &mut cx,
            &layout,
            key("/", PhysicalKey::Slash, KeyPhase::Down),
        );
        assert_eq!(trigger.get(), 1, "page '/' shortcut fires");
        router.route(&mut cx, &layout, key("/", PhysicalKey::Slash, KeyPhase::Up));
        assert_eq!(
            router.focus(),
            Some(field),
            "focus request applies before the next event dispatches"
        );

        router.route(
            &mut cx,
            &layout,
            key("/", PhysicalKey::Slash, KeyPhase::Down),
        );
        assert_eq!(trigger.get(), 1, "shortcut must not re-fire while typing");
        assert_eq!(value.get(), "/");
    });
}
