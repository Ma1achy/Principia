//! The Print graph's port types (TASK-M7-54; principia_print_graph.md §4.2; REQ-RENDER-102): an `image` →
//! `direction` wire is refused with both ports and types named; an unconnected `ground` input of `linocut` takes
//! `paper` and an unconnected `field` input `white`, as the lab's defaults declare. The `linocut` is the fixture's
//! (`fixtures/print_graph/nodes/linocut.json`), as no linocut ships yet. Each test registers its negative control
//! (R-176).

use render::print::graph::{expand, Expanded, Graph};
use render::print::ports::{validate, Feed, InputDefault, PortError, PortType, Wiring};
use render::print::registry::Registry;
use validation::negative_control;

#[path = "support/print_fixtures.rs"]
mod support;

type Validator = fn(&Expanded, &Registry) -> Result<Wiring, PortError>;

fn expanded(text: &str) -> Expanded {
    let graph = Graph::parse(text).unwrap_or_else(|e| panic!("the graph does not parse: {e}"));
    let presets = support::presets();
    expand(&graph, &|n: &str| presets.get(n).cloned())
        .unwrap_or_else(|e| panic!("the graph does not expand: {e}"))
}

/// The render's `image` wired into linocut's `direction` input, `flow`.
const MISMATCHED: &str = r#"{"format":"principia.print/1","name":"M","nodes":{"img":{"type":"render"},
  "cut":{"type":"linocut","in":{"in":"img","flow":"img"}},"out":{"type":"output","in":{"in":"cut"}}},"out":"out"}"#;

/// `validate` refuses the mismatched wire, naming both ports and both types.
fn check_mismatch(validate: Validator) {
    let registry = support::registry();
    let Err(err) = validate(&expanded(MISMATCHED), &registry) else {
        panic!("the image → direction wire was accepted");
    };
    for named in ["`img.out` (image)", "`cut.flow` (direction)"] {
        assert!(
            err.0.contains(named),
            "the refusal does not name {named}: {err}"
        );
    }
}

#[test]
fn print_port_types_image_to_direction_refused() {
    check_mismatch(validate);
    // The same wire into an `image` input is accepted.
    let fine = MISMATCHED
        .replace("\"flow\":\"img\"", "\"in\":\"img\"")
        .replace("{\"in\":\"img\",\"in\":\"img\"}", "{\"in\":\"img\"}");
    validate(&expanded(&fine), &support::registry()).expect("an image → image wire");
}

negative_control!(
    print_port_types_image_to_direction_refused,
    "a validator that accepts every wire",
    expected = "the image → direction wire was accepted",
    check_mismatch(|_, _| Ok(Wiring {
        inputs: Default::default()
    }))
);

/// Render → linocut (only `in` wired) → output.
const UNCONNECTED: &str = r#"{"format":"principia.print/1","name":"U","nodes":{"img":{"type":"render"},
  "cut":{"type":"linocut","in":{"in":"img"}},"out":{"type":"output","in":{"in":"cut"}}},"out":"out"}"#;

/// Linocut's unconnected inputs take its declared defaults: `ground` paper, `field` white, `flow` black.
fn check_defaults(validate: Validator) {
    let wiring =
        validate(&expanded(UNCONNECTED), &support::registry()).expect("the graph validates");
    let cut = &wiring.inputs["cut"];
    let feed = |port: &str| cut.iter().find(|(p, _)| p == port).map(|(_, f)| f.clone());
    let wire = Feed::Wire {
        node: "img".into(),
        port: "out".into(),
    };
    assert_eq!(feed("in"), Some(wire), "`in` does not read the render");
    for (port, default) in [
        ("ground", InputDefault::Paper),
        ("field", InputDefault::White),
        ("flow", InputDefault::Black),
        ("paper", InputDefault::Paper),
    ] {
        assert_eq!(
            feed(port),
            Some(Feed::Default(default)),
            "the unconnected `{port}` does not take its declared default"
        );
    }
    let ports: Vec<&str> = cut.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        ports,
        ["in", "flow", "paper", "ground", "field"],
        "not in declaration order"
    );
}

#[test]
fn print_port_types_unconnected_inputs_take_defaults() {
    check_defaults(validate);
    // An unconnected output input defaults to the render; inside a sub-graph, to its preset node's input.
    let plain =
        r#"{"format":"principia.print/1","name":"P","nodes":{"o":{"type":"output"}},"out":"o"}"#;
    let wiring = validate(&expanded(plain), &support::registry()).expect("validates");
    assert_eq!(
        wiring.inputs["o"],
        [("in".to_owned(), Feed::Default(InputDefault::Render))]
    );
    let sub = r#"{"format":"principia.print/1","name":"S","nodes":{"img":{"type":"render"},
      "b":{"type":"blur","in":{"in":"img"}},"pol":{"type":"preset:Bare","in":{"in":"b"}},
      "out":{"type":"output","in":{"in":"pol"}}},"out":"out"}"#;
    let bare = Graph::parse(
        r#"{"format":"principia.print/1","name":"Bare","nodes":{"m":{"type":"polaroid"},
            "out":{"type":"output","in":{"in":"m"}}},"out":"out"}"#,
    )
    .expect("parses");
    let e = expand(&Graph::parse(sub).expect("parses"), &|_: &str| {
        Some(bare.clone())
    })
    .expect("expands");
    let wiring = validate(&e, &support::registry()).expect("validates");
    let b_out = Feed::Wire {
        node: "b".into(),
        port: "out".into(),
    };
    assert_eq!(
        wiring.inputs["pol/m"],
        [("in".to_owned(), b_out)],
        "the sub-graph does not read its input"
    );
}

negative_control!(
    print_port_types_unconnected_inputs_take_defaults,
    "a validator that reads every unconnected input as black",
    expected = "the unconnected `ground` does not take its declared default",
    check_defaults(|g, r| {
        let mut w = validate(g, r)?;
        for feeds in w.inputs.values_mut() {
            for (_, f) in feeds.iter_mut() {
                if matches!(f, Feed::Default(_)) {
                    *f = Feed::Default(InputDefault::Black);
                }
            }
        }
        Ok(w)
    })
);

/// Each graph of `cases` is refused, the refusal holding the text given with it.
fn check_refusals(cases: &[(&str, &str)]) {
    let registry = support::registry();
    for (text, why) in cases {
        match validate(&expanded(text), &registry) {
            Ok(_) => panic!("a miswired graph validated: {text}"),
            Err(e) => assert!(e.0.contains(why), "the refusal {e} does not say {why:?}"),
        }
    }
}

#[test]
fn print_port_types_other_refusals() {
    let ok = UNCONNECTED;
    validate(&expanded(ok), &support::registry()).expect("validates");
    check_refusals(&[
        (
            &ok.replace("\"linocut\"", "\"linocutt\""),
            "node `cut` names the node type `linocutt`, which is not registered",
        ),
        (
            &ok.replace("{\"in\":\"img\"}", "{\"inn\":\"img\"}"),
            "node `cut` (linocut) has no input `inn`",
        ),
        (
            &ok.replace("{\"in\":\"img\"}", "{\"in\":\"img.colour\"}"),
            "reads `img.colour`, an output `render` does not have",
        ),
        (
            &ok.replace("{\"in\":\"cut\"}", "{\"in\":\"img.out\"}")
                .replace("\"out\":\"out\"}", "\"out\":\"cut\"}"),
            "is a `linocut`, not an `output` node",
        ),
        // The output node has no outputs, so nothing reads it.
        (
            &ok.replace(
                "\"cut\":{\"type\":\"linocut\",\"in\":{\"in\":\"img\"}}",
                "\"cut\":{\"type\":\"linocut\",\"in\":{\"in\":\"out\"}}",
            ),
            "an output `output` does not have",
        ),
    ]);
    // A port named on the wire is checked like the first: `img.out` is the render's image.
    validate(
        &expanded(&ok.replace("{\"in\":\"img\"}", "{\"in\":\"img.out\"}")),
        &support::registry(),
    )
    .expect("validates");
    // Every type has its name, read back.
    for t in PortType::ALL {
        assert_eq!(PortType::parse(t.name()), Some(t));
        assert_eq!(t.to_string(), t.name());
    }
    for d in InputDefault::ALL {
        assert_eq!(InputDefault::parse(d.name()), Some(d));
    }
    assert_eq!(PortType::parse("Image"), None);
    assert_eq!(InputDefault::parse("grey"), None);
}

negative_control!(
    print_port_types_other_refusals,
    "a graph that is wired well",
    expected = "a miswired graph validated",
    check_refusals(&[(UNCONNECTED, "")])
);
